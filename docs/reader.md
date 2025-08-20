# Reader Module

File I/O and streaming abstractions for FASTQ parsing.

## Overview

The reader module provides the main entry point for parsing FASTQ files from various sources. It handles file I/O, compression detection, memory mapping, and streaming operations.

## FastqReader Struct

The primary interface for reading FASTQ files.

```rust
pub struct FastqReader {
    // Internal implementation
}
```

### Constructors

#### from_path

Read a FASTQ file from disk. Automatically detects compression and uses memory mapping for large files.

```rust
pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self>
```

**Example:**
```rust
let reader = FastqReader::from_path("input.fastq")?;
let reader_gz = FastqReader::from_path("input.fastq.gz")?;
```

#### from_bytes

Create a reader from in-memory data.

```rust
pub fn from_bytes(data: Vec<u8>) -> Result<Self>
```

**Example:**
```rust
let data = std::fs::read("input.fastq")?;
let reader = FastqReader::from_bytes(data)?;
```

#### from_mmap

Explicitly use memory-mapped I/O.

```rust
pub fn from_mmap<P: AsRef<Path>>(path: P) -> Result<Self>
```

**Example:**
```rust
let reader = FastqReader::from_mmap("huge.fastq")?;
```

#### from_gzip

Read from a gzip-compressed file.

```rust
pub fn from_gzip<P: AsRef<Path>>(path: P) -> Result<Self>
```

**Example:**
```rust
let reader = FastqReader::from_gzip("compressed.fastq.gz")?;
```

#### streaming

Create a streaming reader for constant memory usage.

```rust
pub fn streaming<P: AsRef<Path>>(path: P) -> Result<Self>
```

**Example:**
```rust
let reader = FastqReader::streaming("huge.fastq")?;
```

### Methods

#### into_records

Convert the reader into an iterator over records.

```rust
pub fn into_records(self) -> RecordIterator
```

**Example:**
```rust
for record in reader.into_records() {
    let record = record?;
    process_record(&record);
}
```

#### iter_records

Iterate over records without consuming the reader.

```rust
pub fn iter_records(&self) -> Result<RecordIterator>
```

#### next_record

Read a single record (for streaming mode).

```rust
pub fn next_record(&mut self) -> Result<Option<FastqRecord>>
```

**Example:**
```rust
let mut reader = FastqReader::streaming("input.fastq")?;
while let Some(record) = reader.next_record()? {
    process_record(&record);
}
```

#### parse_all

Parse all records at once.

```rust
pub fn parse_all(self) -> Result<Vec<FastqRecord>>
```

**Example:**
```rust
let records = reader.parse_all()?;
println!("Parsed {} records", records.len());
```

#### validate

Validate the FASTQ file format without fully parsing.

```rust
pub fn validate(&self) -> Result<()>
```

## RecordIterator

Iterator over FASTQ records.

```rust
pub struct RecordIterator {
    // Internal implementation
}

impl Iterator for RecordIterator {
    type Item = Result<FastqRecord>;
    
    fn next(&mut self) -> Option<Self::Item> {
        // ...
    }
}
```

### Usage

```rust
let reader = FastqReader::from_path("input.fastq")?;
let mut count = 0;

for result in reader.into_records() {
    match result {
        Ok(record) => {
            count += 1;
            // Process record
        }
        Err(e) => {
            eprintln!("Error at record {}: {}", count + 1, e);
            break;
        }
    }
}
```

## ReaderOptions

Configuration options for the reader.

```rust
pub struct ReaderOptions {
    pub buffer_size: usize,
    pub use_mmap: bool,
    pub validate: bool,
    pub parallel: bool,
}

impl Default for ReaderOptions {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            use_mmap: true,
            validate: true,
            parallel: false,
        }
    }
}
```

### Usage

```rust
let options = ReaderOptions {
    buffer_size: 16384,
    use_mmap: false,
    validate: false,
    parallel: true,
};

let reader = FastqReader::with_options("input.fastq", options)?;
```

## File Format Detection

The reader automatically detects:

1. **Compression**: `.gz` extension triggers gzip decompression
2. **File Size**: Large files (>10MB) automatically use memory mapping
3. **Format**: Validates FASTQ format structure

## Memory Management

### Small Files (<10MB)

Files are read entirely into memory for optimal performance.

### Medium Files (10-100MB)

Memory mapping is used by default to avoid large allocations.

### Large Files (>100MB)

Consider using:
- Memory mapping (automatic)
- Streaming mode for constant memory
- Parallel processing for performance

## Error Handling

The reader module defines specific error cases:

```rust
pub enum ReaderError {
    Io(std::io::Error),
    InvalidFormat(String),
    CompressionError(String),
    MmapError(String),
}
```

## Performance Tips

1. **Use memory mapping** for files >10MB
2. **Enable parallel processing** for files >100MB
3. **Use streaming mode** when memory is constrained
4. **Disable validation** for trusted input files
5. **Adjust buffer size** based on record size

## Examples

### Reading Compressed Files

```rust
use fastq_parser::FastqReader;

fn read_compressed(path: &str) -> Result<()> {
    // Automatic detection
    let reader = FastqReader::from_path("data.fastq.gz")?;
    
    for record in reader.into_records() {
        let record = record?;
        // Process compressed data transparently
    }
    
    Ok(())
}
```

### Streaming Large Files

```rust
use fastq_parser::FastqReader;

fn stream_large_file(path: &str) -> Result<()> {
    let mut reader = FastqReader::streaming(path)?;
    let mut count = 0;
    
    while let Some(record) = reader.next_record()? {
        count += 1;
        if count % 1_000_000 == 0 {
            println!("Processed {} million records", count / 1_000_000);
        }
    }
    
    Ok(())
}
```

### Parallel Reading

```rust
use fastq_parser::{FastqReader, ReaderOptions};

fn parallel_read(path: &str) -> Result<()> {
    let options = ReaderOptions {
        parallel: true,
        ..Default::default()
    };
    
    let reader = FastqReader::with_options(path, options)?;
    let records = reader.parse_all()?;
    
    Ok(())
}
```

## See Also

- [Parser Module](./parser.md) - Core parsing logic
- [Record Module](./record.md) - FASTQ record structure
- [Error Module](./error.md) - Error handling