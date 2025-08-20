use std::io::{self, Read};

const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

pub struct BufferedReader<R: Read> {
    reader: R,
    buffer: Vec<u8>,
    pos: usize,
    cap: usize,
    eof: bool,
}

impl<R: Read> BufferedReader<R> {
    pub fn new(reader: R) -> Self {
        Self::with_capacity(DEFAULT_BUFFER_SIZE, reader)
    }
    
    pub fn with_capacity(capacity: usize, reader: R) -> Self {
        BufferedReader {
            reader,
            buffer: vec![0; capacity],
            pos: 0,
            cap: 0,
            eof: false,
        }
    }
    
    #[inline]
    pub fn available(&self) -> usize {
        self.cap - self.pos
    }
    
    #[inline]
    pub fn consumed(&self) -> &[u8] {
        &self.buffer[self.pos..self.cap]
    }
    
    #[inline]
    pub fn consume(&mut self, amt: usize) {
        self.pos = std::cmp::min(self.pos + amt, self.cap);
    }
    
    pub fn fill_buffer(&mut self) -> io::Result<usize> {
        if self.eof {
            return Ok(0);
        }
        
        if self.pos > 0 {
            self.buffer.copy_within(self.pos..self.cap, 0);
            self.cap -= self.pos;
            self.pos = 0;
        }
        
        let bytes_read = self.reader.read(&mut self.buffer[self.cap..])?;
        if bytes_read == 0 {
            self.eof = true;
        }
        self.cap += bytes_read;
        Ok(bytes_read)
    }
    
    pub fn ensure_buffer(&mut self, min_size: usize) -> io::Result<bool> {
        while self.available() < min_size && !self.eof {
            self.fill_buffer()?;
        }
        Ok(self.available() >= min_size)
    }
}

pub struct CircularBuffer {
    buffer: Vec<u8>,
    write_pos: usize,
    read_pos: usize,
    size: usize,
}

impl CircularBuffer {
    pub fn new(capacity: usize) -> Self {
        CircularBuffer {
            buffer: vec![0; capacity],
            write_pos: 0,
            read_pos: 0,
            size: 0,
        }
    }
    
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }
    
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }
    
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
    
    #[inline]
    pub fn is_full(&self) -> bool {
        self.size == self.buffer.len()
    }
    
    pub fn write(&mut self, data: &[u8]) -> usize {
        let available = self.buffer.len() - self.size;
        let to_write = std::cmp::min(data.len(), available);
        
        for &byte in &data[..to_write] {
            self.buffer[self.write_pos] = byte;
            self.write_pos = (self.write_pos + 1) % self.buffer.len();
        }
        
        self.size += to_write;
        to_write
    }
    
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let to_read = std::cmp::min(buf.len(), self.size);
        
        for i in 0..to_read {
            buf[i] = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.buffer.len();
        }
        
        self.size -= to_read;
        to_read
    }
    
    pub fn clear(&mut self) {
        self.write_pos = 0;
        self.read_pos = 0;
        self.size = 0;
    }
}