# Writer Module

The `writer` module provides functionality for writing FASTQ/FASTA files and format conversion.

## Types

### `FastqWriter<W>`

Writes FASTQ records to various outputs with optional compression.

```rust
pub enum FastqWriter<W: Write> {
    Plain(BufWriter<W>),
    Gzip(GzEncoder<BufWriter<W>>),
}
```

#### Methods

- `to_file(path)` - Create writer for file (auto-detects .gz extension)
- `new(writer)` - Create plain writer
- `new_gzip(writer, compression)` - Create compressed writer
- `write_record(&record)` - Write a single record
- `write_owned_record(&record)` - Write an owned record
- `flush()` - Flush buffered data

### `FastaWriter<W>`

Writes FASTA format with configurable line width.

```rust
pub struct FastaWriter<W: Write> {
    writer: BufWriter<W>,
    line_width: usize,
}
```

#### Methods

- `to_file(path)` - Create writer for file
- `line_width(width)` - Set sequence line width (default: 80)
- `write_record(&record)` - Write FASTQ record as FASTA

### `FormatConverter`

Static utilities for format conversion and filtering.

#### Methods

- `fastq_to_fasta(input, output)` - Convert FASTQ to FASTA
- `filter_and_write(input, output, filter_fn)` - Filter and write records

### `SubsetExtractor`

Extract specific subsets of reads.

#### Methods

- `extract_by_ids(input, output, ids)` - Extract reads by ID list
- `extract_range(input, output, start, count)` - Extract range of reads

## Usage Examples

### Writing FASTQ

```rust
use fastq_parser::{FastqWriter, Record};

let mut writer = FastqWriter::to_file("output.fastq")?;

let record = Record::new(b"READ1", None, b"ACGT", b"IIII");
writer.write_record(&record)?;
writer.flush()?;
```

### Automatic Compression

```rust
// Extension .gz triggers automatic compression
let mut writer = FastqWriter::to_file("output.fastq.gz")?;
writer.write_record(&record)?;
```

### Format Conversion

```rust
use fastq_parser::FormatConverter;

// Convert FASTQ to FASTA
let count = FormatConverter::fastq_to_fasta(
    "input.fastq", 
    "output.fasta"
)?;
println!("Converted {} reads", count);
```

### Filtering While Writing

```rust
let (total, passed) = FormatConverter::filter_and_write(
    "input.fastq",
    "filtered.fastq", 
    |record| record.len() >= 100  // Keep reads >= 100bp
)?;
println!("Kept {}/{} reads", passed, total);
```

### Extract Specific Reads

```rust
use fastq_parser::SubsetExtractor;

// Extract by IDs
let ids = vec![b"READ1".to_vec(), b"READ2".to_vec()];
let count = SubsetExtractor::extract_by_ids(
    "input.fastq",
    "subset.fastq",
    &ids
)?;

// Extract range
let count = SubsetExtractor::extract_range(
    "input.fastq",
    "range.fastq",
    100,  // Start at read 100
    50    // Extract 50 reads
)?;
```

## Performance Notes

- Buffered I/O for efficient writing
- Automatic flushing on drop
- Compression level configurable for gzip output
- Supports streaming to avoid loading entire file