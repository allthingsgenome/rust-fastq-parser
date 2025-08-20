use crate::{error::Result, record::OwnedRecord, parser::Parser};
use std::io::{BufRead, BufReader, Read};
use std::collections::VecDeque;

const DEFAULT_BUFFER_SIZE: usize = 8 * 1024 * 1024;
const MIN_BUFFER_RESERVE: usize = 1024 * 1024;

pub struct StreamingReader<R: Read> {
    reader: BufReader<R>,
    buffer: Vec<u8>,
    records_buffer: VecDeque<OwnedRecord>,
    position: usize,
    eof: bool,
}

impl<R: Read> StreamingReader<R> {
    pub fn new(reader: R) -> Self {
        Self::with_capacity(DEFAULT_BUFFER_SIZE, reader)
    }
    
    pub fn with_capacity(capacity: usize, reader: R) -> Self {
        StreamingReader {
            reader: BufReader::with_capacity(capacity, reader),
            buffer: Vec::with_capacity(capacity),
            records_buffer: VecDeque::with_capacity(100),
            position: 0,
            eof: false,
        }
    }
    
    pub fn next_record(&mut self) -> Result<Option<OwnedRecord>> {
        if !self.records_buffer.is_empty() {
            return Ok(self.records_buffer.pop_front());
        }
        
        if self.eof && self.position >= self.buffer.len() {
            return Ok(None);
        }
        
        self.fill_buffer()?;
        self.parse_buffer()?;
        
        Ok(self.records_buffer.pop_front())
    }
    
    fn fill_buffer(&mut self) -> Result<()> {
        if self.eof {
            return Ok(());
        }
        
        if self.position > 0 {
            self.buffer.drain(..self.position);
            self.position = 0;
        }
        
        let available_space = self.buffer.capacity() - self.buffer.len();
        if available_space < MIN_BUFFER_RESERVE {
            self.buffer.reserve(MIN_BUFFER_RESERVE);
        }
        
        let mut temp_buffer = vec![0u8; MIN_BUFFER_RESERVE];
        match self.reader.read(&mut temp_buffer)? {
            0 => self.eof = true,
            n => {
                self.buffer.extend_from_slice(&temp_buffer[..n]);
            }
        }
        
        Ok(())
    }
    
    fn parse_buffer(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        
        let mut last_complete = self.find_last_complete_record();
        
        if last_complete == 0 && !self.eof {
            return Ok(());
        }
        
        if self.eof {
            last_complete = self.buffer.len();
        }
        
        let parse_slice = &self.buffer[self.position..last_complete];
        let mut parser = Parser::new(parse_slice);
        
        while let Some(record) = parser.parse_record()? {
            self.records_buffer.push_back(OwnedRecord::from_record(&record));
        }
        
        self.position = last_complete;
        
        Ok(())
    }
    
    fn find_last_complete_record(&self) -> usize {
        let mut pos = self.buffer.len();
        let mut newline_count = 0;
        
        while pos > self.position && newline_count < 4 {
            pos -= 1;
            if self.buffer[pos] == b'\n' {
                newline_count += 1;
                
                if newline_count >= 3 && pos + 1 < self.buffer.len() && self.buffer[pos + 1] == b'@' {
                    return pos + 1;
                }
            }
        }
        
        self.position
    }
}

impl<R: Read> Iterator for StreamingReader<R> {
    type Item = Result<OwnedRecord>;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.next_record().transpose()
    }
}

pub struct AsyncStreamingReader<R: Read + Send> {
    reader: R,
    buffer_size: usize,
    channel_size: usize,
}

impl<R: Read + Send + 'static> AsyncStreamingReader<R> {
    pub fn new(reader: R) -> Self {
        AsyncStreamingReader {
            reader,
            buffer_size: DEFAULT_BUFFER_SIZE,
            channel_size: 1000,
        }
    }
    
    pub fn with_capacity(buffer_size: usize, channel_size: usize, reader: R) -> Self {
        AsyncStreamingReader {
            reader,
            buffer_size,
            channel_size,
        }
    }
}

impl<R: Read + Send + 'static> IntoIterator for AsyncStreamingReader<R> {
    type Item = Result<OwnedRecord>;
    type IntoIter = ReceiverIterator;
    
    fn into_iter(self) -> Self::IntoIter {
        let (sender, receiver) = std::sync::mpsc::sync_channel(self.channel_size);
        
        std::thread::spawn(move || {
            let stream = StreamingReader::with_capacity(self.buffer_size, self.reader);
            
            for result in stream {
                if sender.send(result).is_err() {
                    break;
                }
            }
        });
        
        ReceiverIterator { receiver }
    }
}

pub struct ReceiverIterator {
    receiver: std::sync::mpsc::Receiver<Result<OwnedRecord>>,
}

impl Iterator for ReceiverIterator {
    type Item = Result<OwnedRecord>;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.recv().ok()
    }
}

pub struct ChunkedStreamer<R: BufRead> {
    reader: R,
    chunk_size: usize,
    overlap: usize,
    buffer: Vec<u8>,
    last_chunk: bool,
}

impl<R: BufRead> ChunkedStreamer<R> {
    pub fn new(reader: R) -> Self {
        Self::with_params(reader, 16 * 1024 * 1024, 1024)
    }
    
    pub fn with_params(reader: R, chunk_size: usize, overlap: usize) -> Self {
        ChunkedStreamer {
            reader,
            chunk_size,
            overlap,
            buffer: Vec::with_capacity(chunk_size + overlap),
            last_chunk: false,
        }
    }
    
    pub fn next_chunk(&mut self) -> Result<Option<Vec<u8>>> {
        if self.last_chunk {
            return Ok(None);
        }
        
        self.buffer.clear();
        
        if self.buffer.capacity() < self.chunk_size {
            self.buffer.reserve(self.chunk_size - self.buffer.len());
        }
        
        let mut total_read = 0;
        let mut temp = vec![0u8; 8192];
        
        while total_read < self.chunk_size {
            match self.reader.read(&mut temp)? {
                0 => {
                    self.last_chunk = true;
                    break;
                }
                n => {
                    self.buffer.extend_from_slice(&temp[..n]);
                    total_read += n;
                }
            }
        }
        
        if self.buffer.is_empty() {
            return Ok(None);
        }
        
        if !self.last_chunk {
            let mut extra_read = 0;
            while extra_read < self.overlap {
                match self.reader.read(&mut temp)? {
                    0 => {
                        self.last_chunk = true;
                        break;
                    }
                    n => {
                        self.buffer.extend_from_slice(&temp[..n]);
                        extra_read += n;
                        
                        if let Some(pos) = self.find_record_boundary(&self.buffer[self.chunk_size..]) {
                            self.buffer.truncate(self.chunk_size + pos);
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(Some(self.buffer.clone()))
    }
    
    fn find_record_boundary(&self, data: &[u8]) -> Option<usize> {
        for (i, window) in data.windows(2).enumerate() {
            if window[0] == b'\n' && window[1] == b'@' {
                return Some(i + 1);
            }
        }
        None
    }
}