# Barcode Module

The `barcode` module provides functionality for molecular barcode and UMI (Unique Molecular Identifier) processing, including extraction, demultiplexing, and deduplication.

## Types

### `BarcodeConfig`

Configuration for barcode/UMI extraction.

```rust
pub struct BarcodeConfig {
    pub barcode_start: usize,
    pub barcode_length: usize,
    pub umi_start: Option<usize>,
    pub umi_length: Option<usize>,
    pub max_mismatches: usize,
    pub in_header: bool,
}
```

#### Methods

- `new(start, length)` - Create with barcode position
- `with_umi(start, length)` - Add UMI configuration
- `max_mismatches(n)` - Set error tolerance
- `in_header(bool)` - Extract from header vs sequence

### `BarcodeExtractor`

Extract barcodes and UMIs from reads.

#### Methods

- `extract(&record)` - Extract barcode and optional UMI
- `extract_and_trim(&record)` - Extract and remove from sequence

### `Demultiplexer`

Demultiplex reads based on barcodes.

#### Methods

- `new(config, barcode_map)` - Create with barcode-to-sample mapping
- `error_correction(bool)` - Enable/disable error correction
- `assign_sample(&barcode)` - Determine sample for barcode
- `demultiplex_to_files(records, output_dir, prefix)` - Demultiplex to separate files

### `UmiDeduplicator`

Remove PCR duplicates based on UMIs.

#### Methods

- `deduplicate(records)` - Remove duplicates based on UMI+sequence
- `min_quality(q)` - Keep highest quality among duplicates

### `BarcodeCorrector`

Error correction for barcodes.

#### Methods

- `new(known_barcodes, max_distance)` - Create with known barcodes
- `correct(&barcode)` - Correct barcode if within distance

## Usage Examples

### Basic Barcode Extraction

```rust
use fastq_parser::{BarcodeConfig, BarcodeExtractor};

let config = BarcodeConfig::new(0, 8)  // 8bp at start
    .with_umi(8, 10);  // 10bp UMI after barcode

let extractor = BarcodeExtractor::new(config);

let (barcode, umi) = extractor.extract(&record).unwrap();
println!("Barcode: {}", String::from_utf8_lossy(&barcode));
if let Some(umi) = umi {
    println!("UMI: {}", String::from_utf8_lossy(&umi));
}
```

### Demultiplexing

```rust
use fastq_parser::{BarcodeConfig, Demultiplexer};
use std::collections::HashMap;

// Map barcodes to sample names
let mut barcodes = HashMap::new();
barcodes.insert(b"ATCGATCG".to_vec(), "Sample1".to_string());
barcodes.insert(b"GCTAGCTA".to_vec(), "Sample2".to_string());

let config = BarcodeConfig::new(0, 8).max_mismatches(1);
let demux = Demultiplexer::new(config, barcodes)
    .error_correction(true);

// Demultiplex to files
let stats = demux.demultiplex_to_files(
    records,
    "output/",
    "experiment"
)?;

stats.print_summary();
```

### UMI Deduplication

```rust
use fastq_parser::UmiDeduplicator;

let dedup = UmiDeduplicator::new()
    .min_quality(30.0);  // Keep best quality

let unique_reads = dedup.deduplicate(records.into_iter());
println!("Unique reads: {}", unique_reads.len());
```

### Barcode Error Correction

```rust
use fastq_parser::BarcodeCorrector;
use std::collections::HashSet;

let mut known = HashSet::new();
known.insert(b"ATCGATCG".to_vec());
known.insert(b"GCTAGCTA".to_vec());

let corrector = BarcodeCorrector::new(known, 1);  // 1 mismatch

let observed = b"ATCGATGG";  // 1 error
if let Some(corrected) = corrector.correct(observed) {
    println!("Corrected: {}", String::from_utf8_lossy(&corrected));
}
```

### Extract and Trim

```rust
// Remove barcode/UMI from sequence after extraction
let (extracted, trimmed_record) = extractor.extract_and_trim(&record);

if let Some((barcode, umi)) = extracted {
    // Process barcode/UMI
    // trimmed_record has barcode/UMI removed from sequence
}
```

## Demultiplexing Output

The demultiplexer creates:
- `prefix_Sample1.fastq` - Reads for Sample1
- `prefix_Sample2.fastq` - Reads for Sample2
- `prefix_undetermined.fastq` - Unassigned reads

UMI and barcode info is added to read headers:
```
@READ1:UMI_ACGTACGT_BC_ATCGATCG
```

## Performance Notes

- Hamming distance for error correction
- HashMap lookup for barcode assignment
- Efficient batch demultiplexing
- UMI deduplication uses HashMap with (UMI, sequence) keys