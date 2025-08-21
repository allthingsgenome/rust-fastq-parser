use crate::{error::Result, record::{Record, OwnedRecord}};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub enum FastqWriter<W: Write> {
    Plain(BufWriter<W>),
    Gzip(GzEncoder<BufWriter<W>>),
}

impl FastqWriter<File> {
    pub fn to_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = File::create(path)?;
        
        if path.extension().and_then(|s| s.to_str()) == Some("gz") {
            Ok(FastqWriter::Gzip(GzEncoder::new(
                BufWriter::new(file),
                Compression::default(),
            )))
        } else {
            Ok(FastqWriter::Plain(BufWriter::new(file)))
        }
    }
}

impl<W: Write> FastqWriter<W> {
    pub fn new(writer: W) -> Self {
        FastqWriter::Plain(BufWriter::new(writer))
    }
    
    pub fn new_gzip(writer: W, compression: Compression) -> Self {
        FastqWriter::Gzip(GzEncoder::new(BufWriter::new(writer), compression))
    }
    
    pub fn write_record(&mut self, record: &Record) -> Result<()> {
        let writer: &mut dyn Write = match self {
            FastqWriter::Plain(w) => w,
            FastqWriter::Gzip(w) => w,
        };
        
        writer.write_all(b"@")?;
        writer.write_all(record.id())?;
        if let Some(desc) = record.desc() {
            writer.write_all(b" ")?;
            writer.write_all(desc)?;
        }
        writer.write_all(b"\n")?;
        writer.write_all(record.seq())?;
        writer.write_all(b"\n+\n")?;
        writer.write_all(record.qual())?;
        writer.write_all(b"\n")?;
        
        Ok(())
    }
    
    pub fn write_owned_record(&mut self, record: &OwnedRecord) -> Result<()> {
        self.write_record(&record.as_record())
    }
    
    pub fn flush(&mut self) -> Result<()> {
        match self {
            FastqWriter::Plain(w) => w.flush()?,
            FastqWriter::Gzip(w) => w.flush()?,
        }
        Ok(())
    }
}

impl<W: Write> Drop for FastqWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

pub struct FastaWriter<W: Write> {
    writer: BufWriter<W>,
    line_width: usize,
}

impl FastaWriter<File> {
    pub fn to_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::create(path)?;
        Ok(FastaWriter::new(file))
    }
}

impl<W: Write> FastaWriter<W> {
    pub fn new(writer: W) -> Self {
        FastaWriter {
            writer: BufWriter::new(writer),
            line_width: 80,
        }
    }
    
    pub fn line_width(mut self, width: usize) -> Self {
        self.line_width = width;
        self
    }
    
    pub fn write_record(&mut self, record: &Record) -> Result<()> {
        self.writer.write_all(b">")?;
        self.writer.write_all(record.id())?;
        if let Some(desc) = record.desc() {
            self.writer.write_all(b" ")?;
            self.writer.write_all(desc)?;
        }
        self.writer.write_all(b"\n")?;
        
        for chunk in record.seq().chunks(self.line_width) {
            self.writer.write_all(chunk)?;
            self.writer.write_all(b"\n")?;
        }
        
        Ok(())
    }
    
    pub fn write_owned_record(&mut self, record: &OwnedRecord) -> Result<()> {
        self.write_record(&record.as_record())
    }
    
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

impl<W: Write> Drop for FastaWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

pub struct FormatConverter;

impl FormatConverter {
    pub fn fastq_to_fasta<P: AsRef<Path>>(input: P, output: P) -> Result<usize> {
        use crate::reader::FastqReader;
        
        let reader = FastqReader::from_path(input)?;
        let mut writer = FastaWriter::to_file(output)?;
        let mut count = 0;
        
        for record in reader.into_records() {
            let record = record?;
            writer.write_owned_record(&record)?;
            count += 1;
        }
        
        writer.flush()?;
        Ok(count)
    }
    
    pub fn filter_and_write<P: AsRef<Path>, F>(
        input: P,
        output: P,
        filter: F,
    ) -> Result<(usize, usize)>
    where
        F: Fn(&Record) -> bool,
    {
        use crate::reader::FastqReader;
        
        let reader = FastqReader::from_path(input)?;
        let mut writer = FastqWriter::to_file(output)?;
        let mut total = 0;
        let mut passed = 0;
        
        for record in reader.into_records() {
            let record = record?;
            total += 1;
            
            if filter(&record.as_record()) {
                writer.write_owned_record(&record)?;
                passed += 1;
            }
        }
        
        writer.flush()?;
        Ok((total, passed))
    }
}

pub struct SubsetExtractor;

impl SubsetExtractor {
    pub fn extract_by_ids<P: AsRef<Path>>(
        input: P,
        output: P,
        ids: &[Vec<u8>],
    ) -> Result<usize> {
        use crate::reader::FastqReader;
        use std::collections::HashSet;
        
        let id_set: HashSet<_> = ids.iter().cloned().collect();
        let reader = FastqReader::from_path(input)?;
        let mut writer = FastqWriter::to_file(output)?;
        let mut count = 0;
        
        for record in reader.into_records() {
            let record = record?;
            if id_set.contains(&record.id) {
                writer.write_owned_record(&record)?;
                count += 1;
            }
        }
        
        writer.flush()?;
        Ok(count)
    }
    
    pub fn extract_range<P: AsRef<Path>>(
        input: P,
        output: P,
        start: usize,
        count: usize,
    ) -> Result<usize> {
        use crate::reader::FastqReader;
        
        let reader = FastqReader::from_path(input)?;
        let mut writer = FastqWriter::to_file(output)?;
        let mut written = 0;
        
        for (i, record) in reader.into_records().enumerate() {
            if i < start {
                continue;
            }
            if written >= count {
                break;
            }
            
            let record = record?;
            writer.write_owned_record(&record)?;
            written += 1;
        }
        
        writer.flush()?;
        Ok(written)
    }
}