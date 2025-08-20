# Record Module

FASTQ record data structures and operations.

## Overview

The record module defines the core `FastqRecord` structure that represents a single FASTQ entry with its four components: identifier, sequence, plus line, and quality scores.

## FastqRecord Struct

The primary data structure for FASTQ entries.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct FastqRecord {
    pub id: Vec<u8>,
    pub seq: Vec<u8>,
    pub plus: Vec<u8>,
    pub qual: Vec<u8>,
}
```

### Constructors

#### new

Create a new FASTQ record.

```rust
impl FastqRecord {
    pub fn new(id: Vec<u8>, seq: Vec<u8>, plus: Vec<u8>, qual: Vec<u8>) -> Self {
        Self { id, seq, plus, qual }
    }
}
```

#### from_strings

Create from string slices.

```rust
pub fn from_strings(id: &str, seq: &str, plus: &str, qual: &str) -> Self {
    Self {
        id: id.as_bytes().to_vec(),
        seq: seq.as_bytes().to_vec(),
        plus: plus.as_bytes().to_vec(),
        qual: qual.as_bytes().to_vec(),
    }
}
```

### Methods

#### validate

Validate the record format.

```rust
pub fn validate(&self) -> Result<()> {
    // Check ID starts with '@'
    if self.id.is_empty() || self.id[0] != b'@' {
        return Err(Error::InvalidHeader);
    }
    
    // Check plus line starts with '+'
    if self.plus.is_empty() || self.plus[0] != b'+' {
        return Err(Error::InvalidPlusLine);
    }
    
    // Check sequence and quality have same length
    if self.seq.len() != self.qual.len() {
        return Err(Error::MismatchedLengths);
    }
    
    Ok(())
}
```

#### id_str

Get the identifier as a string.

```rust
pub fn id_str(&self) -> Result<&str> {
    std::str::from_utf8(&self.id)
        .map_err(|_| Error::InvalidUtf8)
}
```

#### sequence_str

Get the sequence as a string.

```rust
pub fn sequence_str(&self) -> Result<&str> {
    std::str::from_utf8(&self.seq)
        .map_err(|_| Error::InvalidUtf8)
}
```

#### quality_str

Get the quality as a string.

```rust
pub fn quality_str(&self) -> Result<&str> {
    std::str::from_utf8(&self.qual)
        .map_err(|_| Error::InvalidUtf8)
}
```

#### len

Get the sequence length.

```rust
pub fn len(&self) -> usize {
    self.seq.len()
}

pub fn is_empty(&self) -> bool {
    self.seq.is_empty()
}
```

## Zero-Copy Record

A borrowed version for zero-copy parsing.

```rust
#[derive(Debug, Clone, Copy)]
pub struct FastqRecordRef<'a> {
    pub id: &'a [u8],
    pub seq: &'a [u8],
    pub plus: &'a [u8],
    pub qual: &'a [u8],
}

impl<'a> FastqRecordRef<'a> {
    pub fn to_owned(&self) -> FastqRecord {
        FastqRecord {
            id: self.id.to_vec(),
            seq: self.seq.to_vec(),
            plus: self.plus.to_vec(),
            qual: self.qual.to_vec(),
        }
    }
}
```

## Quality Score Operations

### QualityEncoding

Supported quality score encodings.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QualityEncoding {
    Sanger,     // Phred+33 (default)
    Illumina13, // Phred+33 (Illumina 1.3+)
    Illumina15, // Phred+33 (Illumina 1.5+)
    Illumina18, // Phred+33 (Illumina 1.8+)
    Solexa,     // Phred+64 (old Solexa)
    Illumina,   // Phred+64 (old Illumina)
}

impl QualityEncoding {
    pub fn offset(&self) -> u8 {
        match self {
            Self::Sanger | Self::Illumina13 | 
            Self::Illumina15 | Self::Illumina18 => 33,
            Self::Solexa | Self::Illumina => 64,
        }
    }
}
```

### Quality Conversion

Convert between quality encodings.

```rust
impl FastqRecord {
    pub fn to_phred_scores(&self, encoding: QualityEncoding) -> Result<Vec<u8>> {
        let offset = encoding.offset();
        self.qual.iter()
            .map(|&q| {
                if q < offset {
                    Err(Error::InvalidQualityScore(q))
                } else {
                    Ok(q - offset)
                }
            })
            .collect()
    }
    
    pub fn from_phred_scores(phred: &[u8], encoding: QualityEncoding) -> Vec<u8> {
        let offset = encoding.offset();
        phred.iter().map(|&p| p + offset).collect()
    }
    
    pub fn average_quality(&self, encoding: QualityEncoding) -> Result<f32> {
        let phred = self.to_phred_scores(encoding)?;
        let sum: u32 = phred.iter().map(|&q| q as u32).sum();
        Ok(sum as f32 / phred.len() as f32)
    }
}
```

## Sequence Operations

### Nucleotide Counts

Count nucleotides in the sequence.

```rust
impl FastqRecord {
    pub fn nucleotide_counts(&self) -> HashMap<u8, usize> {
        let mut counts = HashMap::new();
        for &nucleotide in &self.seq {
            *counts.entry(nucleotide).or_insert(0) += 1;
        }
        counts
    }
    
    pub fn gc_content(&self) -> f32 {
        let gc_count = self.seq.iter()
            .filter(|&&n| n == b'G' || n == b'C')
            .count();
        gc_count as f32 / self.seq.len() as f32 * 100.0
    }
}
```

### Reverse Complement

Generate reverse complement of the sequence.

```rust
impl FastqRecord {
    pub fn reverse_complement(&self) -> Vec<u8> {
        self.seq.iter()
            .rev()
            .map(|&n| match n {
                b'A' => b'T',
                b'T' => b'A',
                b'C' => b'G',
                b'G' => b'C',
                b'N' => b'N',
                _ => n,
            })
            .collect()
    }
    
    pub fn reverse_complement_record(&self) -> FastqRecord {
        let rc_seq = self.reverse_complement();
        let rc_qual: Vec<u8> = self.qual.iter().rev().copied().collect();
        
        FastqRecord {
            id: self.id.clone(),
            seq: rc_seq,
            plus: self.plus.clone(),
            qual: rc_qual,
        }
    }
}
```

## Trimming Operations

### Quality Trimming

Trim low-quality bases from ends.

```rust
impl FastqRecord {
    pub fn trim_by_quality(&self, min_quality: u8, encoding: QualityEncoding) -> FastqRecord {
        let offset = encoding.offset();
        
        // Find trim points
        let mut start = 0;
        let mut end = self.qual.len();
        
        // Trim from start
        while start < self.qual.len() && self.qual[start] - offset < min_quality {
            start += 1;
        }
        
        // Trim from end
        while end > start && self.qual[end - 1] - offset < min_quality {
            end -= 1;
        }
        
        self.substring(start, end)
    }
}
```

### Adapter Trimming

Remove adapter sequences.

```rust
impl FastqRecord {
    pub fn trim_adapter(&self, adapter: &[u8]) -> FastqRecord {
        if let Some(pos) = self.find_adapter(adapter) {
            self.substring(0, pos)
        } else {
            self.clone()
        }
    }
    
    fn find_adapter(&self, adapter: &[u8]) -> Option<usize> {
        self.seq.windows(adapter.len())
            .position(|window| window == adapter)
    }
}
```

### Length Trimming

Trim to specified length.

```rust
impl FastqRecord {
    pub fn trim_to_length(&self, max_length: usize) -> FastqRecord {
        if self.seq.len() <= max_length {
            self.clone()
        } else {
            self.substring(0, max_length)
        }
    }
    
    fn substring(&self, start: usize, end: usize) -> FastqRecord {
        FastqRecord {
            id: self.id.clone(),
            seq: self.seq[start..end].to_vec(),
            plus: self.plus.clone(),
            qual: self.qual[start..end].to_vec(),
        }
    }
}
```

## Serialization

### To FASTQ Format

```rust
impl FastqRecord {
    pub fn to_fastq_string(&self) -> String {
        format!("{}\n{}\n{}\n{}\n",
            String::from_utf8_lossy(&self.id),
            String::from_utf8_lossy(&self.seq),
            String::from_utf8_lossy(&self.plus),
            String::from_utf8_lossy(&self.qual))
    }
    
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.id)?;
        writer.write_all(b"\n")?;
        writer.write_all(&self.seq)?;
        writer.write_all(b"\n")?;
        writer.write_all(&self.plus)?;
        writer.write_all(b"\n")?;
        writer.write_all(&self.qual)?;
        writer.write_all(b"\n")?;
        Ok(())
    }
}
```

### From FASTQ Format

```rust
impl FromStr for FastqRecord {
    type Err = Error;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lines: Vec<&str> = s.lines().collect();
        
        if lines.len() < 4 {
            return Err(Error::IncompleteRecord);
        }
        
        Ok(FastqRecord {
            id: lines[0].as_bytes().to_vec(),
            seq: lines[1].as_bytes().to_vec(),
            plus: lines[2].as_bytes().to_vec(),
            qual: lines[3].as_bytes().to_vec(),
        })
    }
}
```

## Examples

### Basic Usage

```rust
use fastq_parser::{FastqRecord, QualityEncoding};

fn process_record(record: &FastqRecord) -> Result<()> {
    // Validate
    record.validate()?;
    
    // Get basic info
    println!("ID: {}", record.id_str()?);
    println!("Length: {}", record.len());
    
    // Calculate metrics
    let gc = record.gc_content();
    let avg_qual = record.average_quality(QualityEncoding::Sanger)?;
    
    println!("GC content: {:.1}%", gc);
    println!("Average quality: {:.1}", avg_qual);
    
    Ok(())
}
```

### Quality Filtering

```rust
fn filter_high_quality(records: Vec<FastqRecord>) -> Vec<FastqRecord> {
    records.into_iter()
        .filter(|r| {
            r.average_quality(QualityEncoding::Sanger)
                .unwrap_or(0.0) >= 30.0
        })
        .collect()
}
```

### Adapter Trimming Pipeline

```rust
fn trim_pipeline(record: FastqRecord) -> FastqRecord {
    let adapter = b"AGATCGGAAGAGC";
    
    record
        .trim_adapter(adapter)
        .trim_by_quality(20, QualityEncoding::Sanger)
        .trim_to_length(150)
}
```

## See Also

- [Parser Module](./parser.md) - Creates records
- [Reader Module](./reader.md) - Reads records from files
- [Buffer Module](./buffer.md) - Efficient record storage