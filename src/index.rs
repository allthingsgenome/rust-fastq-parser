use crate::{error::{FastqError, Result}, record::{Record, OwnedRecord}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom};
use std::path::Path;
use memmap2::{Mmap, MmapOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub offset: u64,
    pub length: usize,
    pub seq_length: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FastqIndex {
    entries: HashMap<String, IndexEntry>,
    total_records: usize,
    file_size: u64,
}

impl Default for FastqIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl FastqIndex {
    pub fn new() -> Self {
        FastqIndex {
            entries: HashMap::new(),
            total_records: 0,
            file_size: 0,
        }
    }
    
    pub fn build<P: AsRef<Path>>(fastq_path: P) -> Result<Self> {
        let path = fastq_path.as_ref();
        let file = File::open(path)?;
        let file_size = file.metadata()?.len();
        
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let mut index = FastqIndex::new();
        index.file_size = file_size;
        
        let mut pos = 0;
        let data = &mmap[..];
        
        while pos < data.len() {
            if data[pos] != b'@' {
                return Err(FastqError::InvalidHeader { line: 0 });
            }
            
            let record_start = pos;
            
            let header_end = memchr::memchr(b'\n', &data[pos..])
                .ok_or(FastqError::UnexpectedEof)?;
            let header = &data[pos + 1..pos + header_end];
            
            let id_end = header.iter().position(|&b| b == b' ').unwrap_or(header.len());
            let id = String::from_utf8_lossy(&header[..id_end]).into_owned();
            
            pos += header_end + 1;
            
            let seq_end = memchr::memchr(b'\n', &data[pos..])
                .ok_or(FastqError::UnexpectedEof)?;
            let seq_length = seq_end;
            pos += seq_end + 1;
            
            if data[pos] != b'+' {
                return Err(FastqError::InvalidSeparator { line: 0 });
            }
            
            let sep_end = memchr::memchr(b'\n', &data[pos..])
                .ok_or(FastqError::UnexpectedEof)?;
            pos += sep_end + 1;
            
            let qual_end = memchr::memchr(b'\n', &data[pos..])
                .ok_or(FastqError::UnexpectedEof)?;
            pos += qual_end + 1;
            
            let record_length = pos - record_start;
            
            index.entries.insert(id, IndexEntry {
                offset: record_start as u64,
                length: record_length,
                seq_length,
            });
            
            index.total_records += 1;
        }
        
        Ok(index)
    }
    
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)
            .map_err(|e| FastqError::Io(std::io::Error::other(e)))?;
        Ok(())
    }
    
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        bincode::deserialize_from(reader)
            .map_err(|e| FastqError::Io(std::io::Error::other(e)))
    }
    
    pub fn get(&self, id: &str) -> Option<&IndexEntry> {
        self.entries.get(id)
    }
    
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }
    
    pub fn len(&self) -> usize {
        self.total_records
    }
    
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    
    pub fn ids(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }
}

pub struct IndexedReader {
    mmap: Mmap,
    index: FastqIndex,
}

impl IndexedReader {
    pub fn new<P: AsRef<Path>>(fastq_path: P, index: FastqIndex) -> Result<Self> {
        let file = File::open(fastq_path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        
        Ok(IndexedReader { mmap, index })
    }
    
    pub fn from_paths<P: AsRef<Path>>(fastq_path: P, index_path: P) -> Result<Self> {
        let index = FastqIndex::load(index_path)?;
        Self::new(fastq_path, index)
    }
    
    pub fn get_record(&self, id: &str) -> Option<Record<'_>> {
        let entry = self.index.get(id)?;
        
        if entry.offset as usize + entry.length > self.mmap.len() {
            return None;
        }
        
        let data = &self.mmap[entry.offset as usize..entry.offset as usize + entry.length];
        
        let header_end = memchr::memchr(b'\n', data)?;
        let header = &data[1..header_end];
        
        let (id_bytes, desc) = if let Some(space_pos) = header.iter().position(|&b| b == b' ') {
            (&header[..space_pos], Some(&header[space_pos + 1..]))
        } else {
            (header, None)
        };
        
        let seq_start = header_end + 1;
        let seq_end = seq_start + entry.seq_length;
        let seq = &data[seq_start..seq_end];
        
        let qual_start = data[seq_end..].iter().position(|&b| b == b'\n')? + seq_end + 1;
        let qual_end = qual_start + entry.seq_length;
        let qual = &data[qual_start..qual_end];
        
        Some(Record::new(id_bytes, desc, seq, qual))
    }
    
    pub fn get_owned_record(&self, id: &str) -> Option<OwnedRecord> {
        self.get_record(id).map(|r| OwnedRecord::from_record(&r))
    }
    
    pub fn get_batch(&self, ids: &[&str]) -> Vec<Option<OwnedRecord>> {
        ids.iter().map(|id| self.get_owned_record(id)).collect()
    }
    
    pub fn index(&self) -> &FastqIndex {
        &self.index
    }
    
    pub fn iter_range(&self, start: usize, count: usize) -> RangeIterator<'_> {
        RangeIterator {
            reader: self,
            ids: self.index.ids().skip(start).take(count).cloned().collect(),
            current: 0,
        }
    }
}

pub struct RangeIterator<'a> {
    reader: &'a IndexedReader,
    ids: Vec<String>,
    current: usize,
}

impl<'a> Iterator for RangeIterator<'a> {
    type Item = OwnedRecord;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.ids.len() {
            return None;
        }
        
        let id = &self.ids[self.current];
        self.current += 1;
        
        self.reader.get_owned_record(id)
    }
}

pub struct RandomAccessReader {
    file: BufReader<File>,
    index: FastqIndex,
}

impl RandomAccessReader {
    pub fn new<P: AsRef<Path>>(fastq_path: P, index: FastqIndex) -> Result<Self> {
        let file = File::open(fastq_path)?;
        let reader = BufReader::new(file);
        
        Ok(RandomAccessReader {
            file: reader,
            index,
        })
    }
    
    pub fn from_paths<P: AsRef<Path>>(fastq_path: P, index_path: P) -> Result<Self> {
        let index = FastqIndex::load(index_path)?;
        Self::new(fastq_path, index)
    }
    
    pub fn get_record(&mut self, id: &str) -> Result<Option<OwnedRecord>> {
        let entry = match self.index.get(id) {
            Some(e) => e,
            None => return Ok(None),
        };
        
        self.file.seek(SeekFrom::Start(entry.offset))?;
        
        let mut buffer = vec![0u8; entry.length];
        self.file.read_exact(&mut buffer)?;
        
        let header_end = memchr::memchr(b'\n', &buffer)
            .ok_or(FastqError::UnexpectedEof)?;
        let header = &buffer[1..header_end];
        
        let (id_bytes, desc) = if let Some(space_pos) = header.iter().position(|&b| b == b' ') {
            (&header[..space_pos], Some(&header[space_pos + 1..]))
        } else {
            (header, None)
        };
        
        let seq_start = header_end + 1;
        let seq_end = seq_start + entry.seq_length;
        let seq = &buffer[seq_start..seq_end];
        
        let qual_start = buffer[seq_end..].iter().position(|&b| b == b'\n')
            .ok_or(FastqError::UnexpectedEof)? + seq_end + 1;
        let qual_end = qual_start + entry.seq_length;
        let qual = &buffer[qual_start..qual_end];
        
        Ok(Some(OwnedRecord {
            id: id_bytes.to_vec(),
            desc: desc.map(|d| d.to_vec()),
            seq: seq.to_vec(),
            qual: qual.to_vec(),
        }))
    }
}