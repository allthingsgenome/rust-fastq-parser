# FASTQ Parser API Reference

Complete API documentation for the FASTQ parser library.

## Overview

The FASTQ parser provides a high-performance, feature-rich API for parsing FASTQ files with support for:
- SIMD vectorization for blazing fast performance
- Memory-mapped I/O for efficient large file handling
- Parallel processing capabilities
- Zero-copy parsing to minimize allocations
- Transparent gzip compression support
- Streaming mode for constant memory usage

## Core Modules

### [parser](./parser.md)
Core parsing logic and algorithms. Contains the main `Parser` struct that implements the FASTQ format parsing state machine.

### [reader](./reader.md)
File I/O and streaming abstractions. The `FastqReader` struct provides the main entry point for reading FASTQ files from various sources.

### [parallel](./parallel.md)
Multi-threaded processing capabilities. The `ParallelParser` enables processing large files using multiple CPU cores.

### [simd](./simd.md)
SIMD vectorization for performance. Provides AVX2-optimized functions for common operations like newline detection and character counting.

### [record](./record.md)
FASTQ record data structures. The `FastqRecord` struct represents a single FASTQ entry with its four components.

### [buffer](./buffer.md)
Buffer management and pooling. Provides efficient memory reuse for streaming operations.

### [error](./error.md)
Comprehensive error handling. Defines error types for all failure modes in the parser.

### [mmap](./mmap.md)
Memory-mapped file I/O. Enables efficient handling of large files without loading them entirely into memory.

## Quick Start Examples

### Basic File Parsing

```rust
use fastq_parser::{FastqReader, Result};

fn parse_file(path: &str) -> Result<()> {
    let reader = FastqReader::from_path(path)?;
    
    for record in reader.into_records() {
        let record = record?;
        // Process record
    }
    
    Ok(())
}
```

### Parallel Processing

```rust
use fastq_parser::parallel::ParallelParser;

fn parallel_parse(data: Vec<u8>) -> Result<()> {
    let parser = ParallelParser::new(data);
    
    parser.parse_with_callback(|record| {
        // Process each record in parallel
    })?;
    
    Ok(())
}
```

### SIMD Operations

```rust
use fastq_parser::simd;

fn process_with_simd(data: &[u8]) {
    // Find newlines using AVX2
    let newlines = simd::find_newlines(data);
    
    // Count specific nucleotides
    let a_count = simd::count_chars(data, b'A');
}
```

## Performance Considerations

1. **File Size < 10MB**: Use in-memory parsing for best performance
2. **File Size 10-100MB**: Memory-mapped I/O is automatically enabled
3. **File Size > 100MB**: Consider using parallel processing
4. **Compressed Files**: Streaming decompression minimizes memory usage
5. **Constant Memory**: Use streaming mode for files of any size

## Error Handling

All fallible operations return `Result<T, Error>` where `Error` is the library's error type:

```rust
use fastq_parser::{Result, Error};

match reader.parse() {
    Ok(records) => process(records),
    Err(Error::InvalidFormat(msg)) => {
        eprintln!("Format error: {}", msg);
    }
    Err(Error::Io(e)) => {
        eprintln!("I/O error: {}", e);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Thread Safety

- `FastqReader`: Send + Sync for sharing between threads
- `ParallelParser`: Internally manages thread pool
- `FastqRecord`: Send + Sync for passing between threads
- `BufferPool`: Thread-safe with internal synchronization

## Feature Flags

```toml
[dependencies.fastq-parser]
version = "0.1.0"
features = ["simd", "parallel", "compression"]
```

- `simd`: Enable SIMD optimizations (enabled by default)
- `parallel`: Enable parallel processing (enabled by default)
- `compression`: Enable gzip support (enabled by default)

## Minimum Supported Rust Version

MSRV: 1.70.0

## See Also

- [Parser Module](./parser.md) - Core parsing implementation
- [Reader Module](./reader.md) - File I/O operations
- [Examples](../examples/) - Complete working examples
- [Benchmarks](../benches/) - Performance benchmarks