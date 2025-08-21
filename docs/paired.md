# Paired-End Read Module

The `paired` module provides functionality for handling paired-end sequencing data, supporting both separate R1/R2 files and interleaved formats.

## Types

### `PairedEndReader`

Handles synchronous iteration through R1/R2 file pairs.

```rust
pub struct PairedEndReader {
    r1_reader: FastqReader,
    r2_reader: FastqReader,
}
```

#### Methods

- `from_paths(r1_path, r2_path)` - Create from R1 and R2 file paths
- `into_paired_records()` - Returns iterator of paired records
- `validate_pairing()` - Check if read IDs match between files

### `InterleavedReader`

Handles interleaved FASTQ files where R1/R2 pairs are consecutive.

```rust
pub struct InterleavedReader {
    reader: FastqReader,
}
```

#### Methods

- `from_path(path)` - Create from interleaved file
- `into_paired_records()` - Returns iterator of paired records

## Usage Examples

### Basic Paired-End Processing

```rust
use fastq_parser::PairedEndReader;

let reader = PairedEndReader::from_paths("R1.fastq", "R2.fastq")?;

for pair in reader.into_paired_records() {
    let (r1, r2) = pair?;
    println!("R1: {} ({}bp)", 
             String::from_utf8_lossy(&r1.id), r1.seq.len());
    println!("R2: {} ({}bp)", 
             String::from_utf8_lossy(&r2.id), r2.seq.len());
}
```

### Strict ID Validation

```rust
let reader = PairedEndReader::from_paths("R1.fastq", "R2.fastq")?;

// Enable strict pairing to validate matching IDs
for pair in reader.into_paired_records().strict_pairing(true) {
    let (r1, r2) = pair?; // Will error if IDs don't match
}
```

### Interleaved Format

```rust
use fastq_parser::InterleavedReader;

let reader = InterleavedReader::from_path("interleaved.fastq")?;

for pair in reader.into_paired_records() {
    let (r1, r2) = pair?;
    // Process paired reads from interleaved file
}
```

## Error Handling

The module defines specific errors for paired-end processing:

- `PairedEndMismatch` - Read IDs don't match between R1/R2
- `PairedEndLengthMismatch` - Different number of reads in R1/R2 files
- `InterleavedOddCount` - Odd number of reads in interleaved file

## Performance Notes

- ID matching uses efficient byte comparison after extracting base IDs
- Supports compressed files (gzip) with automatic detection
- Memory-mapped for large files when available