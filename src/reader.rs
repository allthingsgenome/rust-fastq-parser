use crate::{error::Result, parser::{Parser, StreamingParser}, record::{Record, OwnedRecord}};
use flate2::read::MultiGzDecoder;
use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub enum FastqReader {
    Mmap(MmapReader),
    Streaming(Box<dyn Iterator<Item = Result<OwnedRecord>> + Send>),
}

impl FastqReader {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if path.extension().and_then(|s| s.to_str()) == Some("gz") {
            Self::from_gzip_file(path)
        } else {
            Self::from_file(path)
        }
    }
    
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        Ok(FastqReader::Mmap(MmapReader::new(mmap)))
    }
    
    pub fn from_gzip_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let decoder = MultiGzDecoder::new(BufReader::new(file));
        let parser = StreamingParser::new(decoder);
        Ok(FastqReader::Streaming(Box::new(StreamingIterator::new(parser))))
    }
    
    pub fn from_reader<R: Read + Send + 'static>(reader: R) -> Self {
        let parser = StreamingParser::new(reader);
        FastqReader::Streaming(Box::new(StreamingIterator::new(parser)))
    }
    
    pub fn records(&self) -> Box<dyn Iterator<Item = Result<Record<'_>>> + '_> {
        match self {
            FastqReader::Mmap(reader) => Box::new(reader.records()),
            FastqReader::Streaming(_) => panic!("Cannot iterate borrowed records from streaming reader"),
        }
    }
    
    pub fn into_records(self) -> Box<dyn Iterator<Item = Result<OwnedRecord>> + Send> {
        match self {
            FastqReader::Mmap(reader) => Box::new(reader.into_records()),
            FastqReader::Streaming(iter) => iter,
        }
    }
}

pub struct MmapReader {
    mmap: Mmap,
}

impl MmapReader {
    pub fn new(mmap: Mmap) -> Self {
        MmapReader { mmap }
    }
    
    pub fn records(&self) -> impl Iterator<Item = Result<Record<'_>>> + '_ {
        RecordIterator::new(&self.mmap)
    }
    
    pub fn into_records(self) -> impl Iterator<Item = Result<OwnedRecord>> {
        OwnedRecordIterator::new(self.mmap)
    }
}

struct RecordIterator<'a> {
    parser: Parser<'a>,
}

impl<'a> RecordIterator<'a> {
    fn new(data: &'a [u8]) -> Self {
        RecordIterator {
            parser: Parser::new(data),
        }
    }
}

impl<'a> Iterator for RecordIterator<'a> {
    type Item = Result<Record<'a>>;
    
    fn next(&mut self) -> Option<Self::Item> {
        // Prefetch the next cache line for better performance
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::_mm_prefetch;
            if self.parser.pos + 64 < self.parser.data.len() {
                _mm_prefetch(
                    self.parser.data[self.parser.pos + 64..].as_ptr() as *const i8,
                    1  // _MM_HINT_T1
                );
            }
        }
        self.parser.parse_record().transpose()
    }
}

struct OwnedRecordIterator {
    _mmap: Mmap,
    parser: *mut Parser<'static>,
}

impl OwnedRecordIterator {
    fn new(mmap: Mmap) -> Self {
        let data = unsafe { std::slice::from_raw_parts(mmap.as_ptr(), mmap.len()) };
        let parser = Box::new(Parser::new(unsafe { std::mem::transmute::<&[u8], &[u8]>(data) }));
        OwnedRecordIterator {
            _mmap: mmap,
            parser: Box::into_raw(parser),
        }
    }
}

impl Iterator for OwnedRecordIterator {
    type Item = Result<OwnedRecord>;
    
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            (*self.parser).parse_record()
                .map(|opt| opt.map(|r| OwnedRecord::from_record(&r)))
                .transpose()
        }
    }
}

impl Drop for OwnedRecordIterator {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.parser);
        }
    }
}

unsafe impl Send for OwnedRecordIterator {}

struct StreamingIterator<R: Read> {
    parser: StreamingParser<R>,
}

impl<R: Read> StreamingIterator<R> {
    fn new(parser: StreamingParser<R>) -> Self {
        StreamingIterator { parser }
    }
}

impl<R: Read> Iterator for StreamingIterator<R> {
    type Item = Result<OwnedRecord>;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.parser.parse_next().transpose()
    }
}

pub struct FastqReaderBuilder {
    buffer_size: usize,
    parallel: bool,
}

impl Default for FastqReaderBuilder {
    fn default() -> Self {
        FastqReaderBuilder {
            buffer_size: 64 * 1024,
            parallel: false,
        }
    }
}

impl FastqReaderBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }
    
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }
    
    pub fn from_path<P: AsRef<Path>>(&self, path: P) -> Result<FastqReader> {
        FastqReader::from_path(path)
    }
    
    pub fn from_reader<R: Read + Send + 'static>(&self, reader: R) -> FastqReader {
        let parser = StreamingParser::with_capacity(self.buffer_size, reader);
        FastqReader::Streaming(Box::new(StreamingIterator::new(parser)))
    }
}