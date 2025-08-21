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
- Paired-end read synchronization
- Advanced filtering and quality control
- Barcode/UMI processing and demultiplexing
- Index-based random access
- Format conversion (FASTQ to FASTA)
- Comprehensive quality metrics

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

### [filter](./filter.md)
Advanced filtering capabilities. Provides quality filtering, adapter trimming, and content-based filtering with configurable parameters.

### [paired](./paired.md)
Paired-end read handling. Synchronous iteration through R1/R2 file pairs with ID validation and mismatch detection.

### [writer](./writer.md)
FASTQ/FASTA writing and format conversion. Efficient output writing with compression support and format conversion utilities.

### [index](./index.md)
Index-based random access. Build persistent indexes for O(1) lookup of specific reads in large FASTQ files.

### [barcode](./barcode.md)
Barcode and UMI processing. Extract, demultiplex, and deduplicate reads based on molecular barcodes with error correction.

### [metrics](./metrics.md)
Quality metrics and analysis. Calculate per-position quality distributions, duplicate rates, and k-mer based error detection.

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

### Paired-End Processing

```rust
use fastq_parser::PairedEndReader;

fn process_paired_reads(r1: &str, r2: &str) -> Result<()> {
    let reader = PairedEndReader::from_paths(r1, r2)?;
    
    for pair in reader.into_paired_records() {
        let (read1, read2) = pair?;
        // Process paired reads together
    }
    
    Ok(())
}
```

### Advanced Filtering

```rust
use fastq_parser::AdvancedFilter;

fn filter_reads(records: impl Iterator<Item = Record>) {
    let filter = AdvancedFilter::new()
        .min_length(50)
        .max_n_ratio(0.1);
    
    for record in records {
        if filter.filter(&record) {
            // Process passing reads
        }
    }
}
```

### Barcode Demultiplexing

```rust
use fastq_parser::{BarcodeConfig, Demultiplexer};

fn demultiplex(records: impl Iterator<Item = Result<OwnedRecord>>) -> Result<()> {
    let config = BarcodeConfig::new(0, 8);
    let demux = Demultiplexer::new(config, barcode_map);
    
    let stats = demux.demultiplex_to_files(records, "output/", "sample")?;
    stats.print_summary();
    
    Ok(())
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
- [Paired Module](./paired.md) - Paired-end read handling
- [Filter Module](./filter.md) - Advanced filtering
- [Writer Module](./writer.md) - Output and conversion
- [Index Module](./index.md) - Random access indexing
- [Barcode Module](./barcode.md) - Barcode/UMI processing
- [Metrics Module](./metrics.md) - Quality metrics
- [Examples](../examples/) - Complete working examples
- [Benchmarks](../benches/) - Performance benchmarks