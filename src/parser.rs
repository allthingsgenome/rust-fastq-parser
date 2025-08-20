use crate::{error::{FastqError, Result}, record::Record};
use std::io::Read;

pub struct Parser<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) pos: usize,
    line: usize,
}

impl<'a> Parser<'a> {
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        Parser {
            data,
            pos: 0,
            line: 1,
        }
    }
    
    #[inline]
    fn _peek(&self) -> Option<u8> {
        if self.pos < self.data.len() {
            Some(self.data[self.pos])
        } else {
            None
        }
    }
    
    #[inline]
    fn _advance(&mut self, n: usize) {
        self.pos = std::cmp::min(self.pos + n, self.data.len());
    }
    
    #[inline]
    fn find_newline(&self) -> Option<usize> {
        crate::simd::find_char(self.data, b'\n', self.pos)
    }
    
    #[inline]
    fn read_line(&mut self) -> Result<&'a [u8]> {
        let start = self.pos;
        
        if let Some(end) = self.find_newline() {
            self.pos = end + 1;
            self.line += 1;
            
            let mut line_end = end;
            if line_end > start && self.data[line_end - 1] == b'\r' {
                line_end -= 1;
            }
            
            Ok(&self.data[start..line_end])
        } else if self.pos < self.data.len() {
            let end = self.data.len();
            self.pos = end;
            Ok(&self.data[start..end])
        } else {
            Err(FastqError::UnexpectedEof)
        }
    }
    
    pub fn parse_record(&mut self) -> Result<Option<Record<'a>>> {
        if self.pos >= self.data.len() {
            return Ok(None);
        }
        
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_whitespace() {
            if self.data[self.pos] == b'\n' {
                self.line += 1;
            }
            self.pos += 1;
        }
        
        if self.pos >= self.data.len() {
            return Ok(None);
        }
        
        let header_line = self.read_line()?;
        if header_line.is_empty() {
            return Ok(None);
        }
        
        if header_line[0] != b'@' {
            return Err(FastqError::InvalidHeader { line: self.line - 1 });
        }
        
        let (id, desc) = Self::parse_header(&header_line[1..])?;
        
        let seq = self.read_sequence()?;
        
        let sep_line = self.read_line()?;
        if sep_line.is_empty() || sep_line[0] != b'+' {
            return Err(FastqError::InvalidSeparator { line: self.line - 1 });
        }
        
        let qual = self.read_quality(seq.len())?;
        
        if seq.len() != qual.len() {
            return Err(FastqError::LengthMismatch {
                seq_len: seq.len(),
                qual_len: qual.len(),
            });
        }
        
        Ok(Some(Record::new(id, desc, seq, qual)))
    }
    
    fn read_sequence(&mut self) -> Result<&'a [u8]> {
        let start = self.pos;
        let mut seq_end = start;
        
        // Fast path: look for the separator line
        while let Some(newline_pos) = crate::simd::find_char(self.data, b'\n', self.pos) {
            self.pos = newline_pos + 1;
            self.line += 1;
            
            if self.pos < self.data.len() && self.data[self.pos] == b'+' {
                // Update seq_end to exclude trailing whitespace
                for i in (start..newline_pos).rev() {
                    if !self.data[i].is_ascii_whitespace() {
                        seq_end = i + 1;
                        break;
                    }
                }
                if seq_end == start {
                    // Check if there was any non-whitespace content
                    for i in start..newline_pos {
                        if !self.data[i].is_ascii_whitespace() {
                            seq_end = newline_pos;
                            break;
                        }
                    }
                }
                break;
            } else {
                // This line is part of the sequence
                seq_end = newline_pos;
            }
        }
        
        if seq_end == start {
            return Err(FastqError::UnexpectedEof);
        }
        
        Ok(&self.data[start..seq_end])
    }
    
    fn read_quality(&mut self, expected_len: usize) -> Result<&'a [u8]> {
        let start = self.pos;
        let mut quality_end = start;
        let mut non_ws_count = 0;
        
        // Fast scan to find enough quality characters
        let slice = &self.data[start..];
        for (i, &byte) in slice.iter().enumerate() {
            if byte == b'\n' {
                self.line += 1;
            } else if !byte.is_ascii_whitespace() {
                non_ws_count += 1;
                quality_end = start + i + 1;
                if non_ws_count == expected_len {
                    self.pos = quality_end;
                    break;
                }
            }
        }
        
        if non_ws_count != expected_len {
            return Err(FastqError::LengthMismatch {
                seq_len: expected_len,
                qual_len: non_ws_count,
            });
        }
        
        // Skip any trailing whitespace
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_whitespace() {
            if self.data[self.pos] == b'\n' {
                self.line += 1;
                self.pos += 1;
                break;
            }
            self.pos += 1;
        }
        
        Ok(&self.data[start..quality_end])
    }
    
    #[inline]
    fn trim_whitespace(data: &[u8]) -> &[u8] {
        // Fast path for common case: no leading whitespace
        if data.is_empty() || !data[0].is_ascii_whitespace() {
            // Just find the end
            let end = data.iter().rposition(|&b| !b.is_ascii_whitespace())
                .map(|i| i + 1)
                .unwrap_or(0);
            return &data[..end];
        }
        
        // Full trim needed
        let start = data.iter().position(|&b| !b.is_ascii_whitespace()).unwrap_or(0);
        let end = data.iter().rposition(|&b| !b.is_ascii_whitespace()).map(|i| i + 1).unwrap_or(0);
        &data[start..end]
    }
    
    #[inline]
    fn parse_header(header: &[u8]) -> Result<(&[u8], Option<&[u8]>)> {
        // Use SIMD-accelerated character search
        if let Some(space_pos) = crate::simd::find_char(header, b' ', 0) {
            Ok((&header[..space_pos], Some(&header[space_pos + 1..])))
        } else if let Some(tab_pos) = crate::simd::find_char(header, b'\t', 0) {
            Ok((&header[..tab_pos], Some(&header[tab_pos + 1..])))
        } else {
            Ok((header, None))
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Record<'a>;
    
    fn next(&mut self) -> Option<Self::Item> {
        match self.parse_record() {
            Ok(Some(record)) => Some(record),
            Ok(None) => None,
            Err(_) => None,
        }
    }
}

pub struct ParserBuilder {
    validate: bool,
    parallel: bool,
    buffer_size: usize,
}

impl Default for ParserBuilder {
    fn default() -> Self {
        ParserBuilder {
            validate: true,
            parallel: false,
            buffer_size: 64 * 1024,
        }
    }
}

impl ParserBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn validate(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }
    
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }
    
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }
    
    pub fn build<'a>(&self, data: &'a [u8]) -> Parser<'a> {
        Parser::new(data)
    }
}

pub struct StreamingParser<R: Read> {
    reader: crate::buffer::BufferedReader<R>,
}

impl<R: Read> StreamingParser<R> {
    pub fn new(reader: R) -> Self {
        StreamingParser {
            reader: crate::buffer::BufferedReader::new(reader),
        }
    }
    
    pub fn with_capacity(capacity: usize, reader: R) -> Self {
        StreamingParser {
            reader: crate::buffer::BufferedReader::with_capacity(capacity, reader),
        }
    }
    
    pub fn parse_next(&mut self) -> Result<Option<crate::record::OwnedRecord>> {
        self.reader.ensure_buffer(4)?;
        
        let buffer = self.reader.consumed();
        if buffer.is_empty() {
            return Ok(None);
        }
        
        let mut parser = Parser::new(buffer);
        if let Some(record) = parser.parse_record()? {
            let owned = crate::record::OwnedRecord::from_record(&record);
            self.reader.consume(parser.pos);
            Ok(Some(owned))
        } else {
            Ok(None)
        }
    }
}