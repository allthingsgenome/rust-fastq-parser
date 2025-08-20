# FASTQ Parser - High-Performance FASTQ File Parser in Rust

A blazingly fast, feature-rich FASTQ parser written in Rust with SIMD vectorization, memory-mapped I/O, and parallel processing capabilities.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Core API](#core-api)
  - [Reading & Parsing](#reading--parsing)
  - [Parallel Processing](#parallel-processing)
  - [SIMD Operations](#simd-operations)
  - [Memory-Mapped Files](#memory-mapped-files)
  - [Record Processing](#record-processing)
  - [Error Handling](#error-handling)
  - [Buffer Management](#buffer-management)
- [Advanced Usage](#advanced-usage)
  - [Zero-Copy Parsing](#zero-copy-parsing)
  - [Compression Support](#compression-support)
  - [Streaming Mode](#streaming-mode)
- [Examples](#examples)
- [Performance](#performance)
- [API Reference](#api-reference)
- [Development](#development)
- [License](#license)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
fastq-parser = "0.1.0"
```

Or using `pixi` for environment management:

```bash
# Install pixi
curl -fsSL https://pixi.sh/install.sh | bash

# Install dependencies
pixi install

# Build the project
pixi run cargo build --release
```

## Quick Start

```rust
use fastq_parser::{FastqReader, Result};

fn main() -> Result<()> {
    // Parse FASTQ file
    let reader = FastqReader::from_path("input.fastq")?;
    
    // Iterate over records
    for record in reader.into_records() {
        let record = record?;
        println!("ID: {}", std::str::from_utf8(&record.id)?);
        println!("Sequence length: {}", record.seq.len());
    }
    
    Ok(())
}
```

## Core API

### Reading & Parsing

**[`FastqReader`](./docs/reader.md)** - Main entry point for parsing FASTQ files

```rust
use fastq_parser::{FastqReader, Parser, Result};

// Basic file reading
let reader = FastqReader::from_path("input.fastq")?;

// Parse gzipped files
let reader = FastqReader::from_path("input.fastq.gz")?;

// Parse from bytes
let data = std::fs::read("input.fastq")?;
let reader = FastqReader::from_bytes(data)?;

// Streaming iteration (constant memory)
for record in reader.into_records() {
    let r = record?;
    // Process one record at a time
}
```
[📖 Full Reader Documentation](./docs/reader.md)

### Parallel Processing

**[`ParallelParser`](./docs/parallel.md)** - Multi-threaded parsing for large files

```rust
use fastq_parser::parallel::ParallelParser;

// Create parallel parser
let parser = ParallelParser::new(data);

// Process records in parallel with callback
parser.parse_with_callback(|record| {
    // Process each record
    println!("Processing: {}", std::str::from_utf8(&record.id)?);
})?;

// Automatic chunk boundary detection
let results = parser.parse_parallel()?;
```
[📖 Full Parallel Documentation](./docs/parallel.md)

### SIMD Operations

**[`simd`](./docs/simd.md)** - Vectorized operations using AVX2

```rust
use fastq_parser::simd;

// Find all newline positions (uses AVX2 if available)
let positions = simd::find_newlines(data);

// Validate ASCII content
let is_valid = simd::validate_ascii(data);

// Count specific characters
let count = simd::count_chars(data, b'A');

// Vectorized quality score processing
let avg_quality = simd::average_quality(quality_scores);
```
[📖 Full SIMD Documentation](./docs/simd.md)

### Memory-Mapped Files

**[`mmap`](./docs/mmap.md)** - Efficient handling of large files

```rust
use fastq_parser::{FastqReader, MmapOptions};

// Automatic memory mapping for large files
let reader = FastqReader::from_path("huge.fastq")?;

// Manual memory-mapped reading
let mmap_reader = FastqReader::from_mmap("huge.fastq")?;

// Configure mmap options
let options = MmapOptions::new()
    .prefetch(true)
    .sequential_access(true);
```
[📖 Full Memory-Mapping Documentation](./docs/mmap.md)

### Record Processing

**[`FastqRecord`](./docs/record.md)** - Core data structure for FASTQ entries

```rust
use fastq_parser::{FastqRecord, QualityEncoding};

// Access record components
let id = std::str::from_utf8(&record.id)?;
let sequence = &record.seq;
let quality = &record.qual;

// Quality score conversion
let phred_scores = record.to_phred_scores(QualityEncoding::Sanger)?;

// Record validation
let is_valid = record.validate()?;
```
[📖 Full Record Documentation](./docs/record.md)

### Error Handling

**[`Error`](./docs/error.md)** - Comprehensive error types

```rust
use fastq_parser::{Result, Error};

match reader.parse() {
    Ok(records) => process(records),
    Err(Error::InvalidFormat(msg)) => eprintln!("Format error: {}", msg),
    Err(Error::Io(e)) => eprintln!("I/O error: {}", e),
    Err(e) => eprintln!("Other error: {}", e),
}
```
[📖 Full Error Documentation](./docs/error.md)

### Buffer Management

**[`BufferPool`](./docs/buffer.md)** - Efficient memory reuse

```rust
use fastq_parser::buffer::BufferPool;

// Create buffer pool for streaming
let pool = BufferPool::new(capacity);

// Reuse buffers across iterations
let buffer = pool.get();
// ... use buffer ...
pool.return(buffer);
```
[📖 Full Buffer Documentation](./docs/buffer.md)

## Advanced Usage

### Zero-Copy Parsing

Minimize allocations with zero-copy techniques:

```rust
// Parser returns views into original data
let parser = Parser::new();
let records = parser.parse_zero_copy(&data)?;

// No string allocations - work with byte slices
for record in records {
    // record.id, record.seq, record.qual are all &[u8]
}
```

### Compression Support

Native support for gzip-compressed files:

```rust
// Automatic detection based on file extension
let reader = FastqReader::from_path("input.fastq.gz")?;

// Explicit compressed reading
let reader = FastqReader::from_gzip("compressed.gz")?;

// Streaming decompression
for record in reader.iter_compressed()? {
    // Process decompressed records on-the-fly
}
```

### Streaming Mode

Process files of any size with constant memory:

```rust
// Stream large files without loading into memory
let mut reader = FastqReader::streaming("huge.fastq")?;

while let Some(record) = reader.next_record()? {
    // Process one record at a time
    // Memory usage remains constant
}
```

## Examples

### Complete Processing Pipeline

```rust
use fastq_parser::*;

fn process_fastq(path: &str) -> Result<()> {
    // 1. Create reader with optimal settings
    let reader = FastqReader::from_path(path)?;
    
    // 2. Use parallel processing for large files
    if std::fs::metadata(path)?.len() > 100_000_000 {
        let parser = ParallelParser::from_file(path)?;
        parser.parse_with_callback(|record| {
            // Process in parallel
            analyze_record(&record)
        })?;
    } else {
        // 3. Stream smaller files
        for record in reader.into_records() {
            analyze_record(&record?)?;
        }
    }
    
    Ok(())
}

fn analyze_record(record: &FastqRecord) -> Result<()> {
    // Calculate metrics
    let gc_content = calculate_gc_content(&record.seq);
    let avg_quality = calculate_average_quality(&record.qual)?;
    
    // Filter by quality
    if avg_quality >= 30.0 {
        // Process high-quality reads
    }
    
    Ok(())
}
```

### Run Examples

```bash
# Basic usage
pixi run cargo run --example basic_usage

# Parallel processing demonstration
pixi run cargo run --example parallel

# Streaming large files
pixi run cargo run --example streaming

# SIMD operations showcase
pixi run cargo run --example simd_demo
```

## Performance

### Benchmarks

Run benchmarks:

```bash
pixi run cargo bench
```

### Performance Characteristics

| File Size | Method | Throughput | Memory |
|-----------|--------|------------|--------|
| <10 MB | In-memory | 3+ GB/s | Low |
| 10-100 MB | Memory-mapped | 3+ GB/s | Medium |
| >100 MB | Parallel + mmap | 3+ GB/s × cores | Medium |
| Any size | Streaming | 1-2 GB/s | Constant |
| Gzipped | Decompression | ~500 MB/s | Low |

### Optimizations

1. **SIMD Vectorization**: AVX2 instructions for 32-byte parallel processing
2. **Zero-Copy Parsing**: Return views into original data
3. **Memory Mapping**: Direct file access without heap allocation
4. **Parallel Chunking**: Intelligent splitting at record boundaries
5. **Buffer Pooling**: Reuse allocations in streaming mode
6. **Link-Time Optimization**: LTO enabled for release builds

### Performance Metrics

Based on our benchmarks:
- **3+ GB/s** throughput on uncompressed files
- **~450ms** to parse 5.6M records (comparable to needletail)
- **Linear scaling** with multiple CPU cores
- **Constant memory** streaming for files of any size

## API Reference

Complete API documentation is organized by module:

### 📚 [Complete API Documentation](./docs/api-reference.md)

### Core Modules

| Module | Description | Documentation |
|--------|-------------|---------------|
| **[parser](./docs/parser.md)** | Core parsing logic and algorithms | [📖 Docs](./docs/parser.md) |
| **[reader](./docs/reader.md)** | File I/O and streaming | [📖 Docs](./docs/reader.md) |
| **[parallel](./docs/parallel.md)** | Multi-threaded processing | [📖 Docs](./docs/parallel.md) |
| **[simd](./docs/simd.md)** | SIMD vectorization | [📖 Docs](./docs/simd.md) |
| **[record](./docs/record.md)** | FASTQ record structures | [📖 Docs](./docs/record.md) |
| **[buffer](./docs/buffer.md)** | Buffer management | [📖 Docs](./docs/buffer.md) |
| **[error](./docs/error.md)** | Error handling | [📖 Docs](./docs/error.md) |
| **[mmap](./docs/mmap.md)** | Memory-mapped I/O | [📖 Docs](./docs/mmap.md) |

### Quick Links to Key Types

- [`FastqReader`](./docs/reader.md#fastqreader-struct) - Main entry point for parsing
- [`FastqRecord`](./docs/record.md#fastqrecord-struct) - Single FASTQ entry
- [`Parser`](./docs/parser.md#parser-struct) - Core parsing engine
- [`ParallelParser`](./docs/parallel.md#parallelparser-struct) - Multi-threaded parser
- [`BufferPool`](./docs/buffer.md#bufferpool-struct) - Memory management
- [`Error`](./docs/error.md#error-enum) - Error types

## Development

### Setup

Using pixi for environment management:

```bash
# Install dependencies
pixi install

# Run tests
pixi run cargo test

# Run specific test
pixi run cargo test test_parallel_parser

# Check code
pixi run cargo check
pixi run cargo clippy

# Format code
pixi run cargo fmt

# Build documentation
pixi run cargo doc --open

# Run benchmarks
pixi run cargo bench
```

### Project Structure

```
fastq-parser/
├── src/
│   ├── lib.rs         # Public API exports
│   ├── main.rs        # CLI application
│   ├── parser.rs      # Core parsing logic
│   ├── simd.rs        # SIMD optimizations
│   ├── parallel.rs    # Parallel processing
│   ├── reader.rs      # File I/O abstractions
│   ├── buffer.rs      # Buffer management
│   ├── record.rs      # Record structures
│   └── error.rs       # Error handling
├── benches/
│   └── parser_bench.rs # Performance benchmarks
├── tests/
│   └── integration_tests.rs
├── examples/          # Example usage
├── docs/             # Module documentation
├── Cargo.toml
├── pixi.toml         # Pixi environment config
└── README.md
```

### Contributing

Contributions are welcome! Please ensure:
- All tests pass: `pixi run cargo test`
- Code follows Rust formatting: `pixi run cargo fmt`
- No clippy warnings: `pixi run cargo clippy`
- New features include tests
- Performance improvements include benchmarks

## License

MIT License

## Acknowledgments

This parser was inspired by:
- [needletail](https://github.com/onecodex/needletail) - Fast FASTX parsing in Rust
- [seq_io](https://github.com/markschl/seq_io) - FASTA/FASTQ parsing
- [kseq.h](https://github.com/lh3/seqtk) - Classic C implementation
- [biofast benchmarks](https://github.com/lh3/biofast) - Bioinformatics benchmarking