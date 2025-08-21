# Metrics Module

The `metrics` module provides comprehensive quality metrics and analysis for FASTQ data.

## Types

### `QualityMetrics`

Accumulates various quality metrics across reads.

```rust
pub struct QualityMetrics {
    position_qualities: Vec<Vec<u8>>,
    duplicate_tracker: DuplicateTracker,
    kmer_counter: KmerCounter,
    // ... statistics fields
}
```

#### Methods

- `new()` - Create new metrics accumulator
- `update(&mut record)` - Update metrics with a record
- `finalize()` - Compute final statistics
- `position_quality_stats()` - Get per-position quality statistics
- `duplicate_rate()` - Calculate duplicate rate
- `kmer_distribution()` - Get k-mer frequency distribution
- `error_kmers(threshold)` - Find potential error k-mers
- `summary()` - Get summary statistics
- `print_summary()` - Print formatted summary

### `ErrorDetector`

Detect potential sequencing errors using k-mer analysis.

```rust
pub struct ErrorDetector {
    kmer_size: usize,
    min_frequency: usize,
    error_threshold: f64,
}
```

#### Methods

- `kmer_size(k)` - Set k-mer size (default: 5)
- `min_frequency(n)` - Minimum k-mer frequency for valid
- `error_threshold(t)` - Error detection threshold
- `detect_errors(&seq, &kmer_counts)` - Find error positions

### `QualityPlotter`

Generate ASCII visualizations of quality data.

#### Methods

- `generate_ascii_plot(&stats, width, height)` - Create quality plot

## Data Structures

### `PositionStats`

Per-position quality statistics.

```rust
pub struct PositionStats {
    pub position: usize,
    pub mean: f64,
    pub median: u8,
    pub q25: u8,      // 25th percentile
    pub q75: u8,      // 75th percentile
    pub min: u8,
    pub max: u8,
}
```

### `MetricsSummary`

Overall metrics summary.

```rust
pub struct MetricsSummary {
    pub total_reads: usize,
    pub total_bases: usize,
    pub min_length: usize,
    pub max_length: usize,
    pub mean_length: f64,
    pub mean_gc: f64,
    pub n_base_percent: f64,
    pub duplicate_rate: f64,
}
```

## Usage Examples

### Calculate Quality Metrics

```rust
use fastq_parser::{QualityMetrics, FastqReader};

let reader = FastqReader::from_path("reads.fastq")?;
let mut metrics = QualityMetrics::new();

for record in reader.into_records() {
    let mut record = record?;
    metrics.update(&mut record.as_record());
}

metrics.finalize();
metrics.print_summary();
```

### Per-Position Quality Analysis

```rust
let pos_stats = metrics.position_quality_stats();

for stat in pos_stats.iter().take(10) {
    println!("Position {}: mean={:.1}, median={}, [{}-{}]",
             stat.position + 1,
             stat.mean,
             stat.median,
             stat.q25,
             stat.q75);
}
```

### ASCII Quality Plot

```rust
use fastq_parser::QualityPlotter;

let plot = QualityPlotter::generate_ascii_plot(
    &pos_stats,
    60,  // width
    20   // height
);
println!("{}", plot);
```

Output example:
```
Quality Score Distribution (* = mean, o = median)
 40 |    *****
 35 |  ********  
 30 | **********ooooo
 25 | ****************ooooooo
 20 |-------------------------
    Position in read →
```

### Error Detection

```rust
use fastq_parser::ErrorDetector;

let detector = ErrorDetector::new()
    .kmer_size(5)
    .min_frequency(10);

let kmer_counts = metrics.kmer_distribution();
let errors = detector.detect_errors(&sequence, kmer_counts);

for error in errors {
    println!("Potential error at position {}: {} -> {} (confidence: {:.2})",
             error.position,
             error.incorrect_base as char,
             error.suggested_base as char,
             error.confidence);
}
```

### Duplicate Analysis

```rust
let dup_rate = metrics.duplicate_rate();
let exact_dups = metrics.exact_duplicates();

println!("Duplicate rate: {:.2}%", dup_rate * 100.0);
println!("Exact duplicates: {}", exact_dups);
```

### Get Summary Statistics

```rust
let summary = metrics.summary();

println!("Total reads: {}", summary.total_reads);
println!("Total bases: {}", summary.total_bases);
println!("Length range: {}-{} (mean: {:.1})",
         summary.min_length,
         summary.max_length,
         summary.mean_length);
println!("GC content: {:.2}%", summary.mean_gc);
println!("N-bases: {:.4}%", summary.n_base_percent);
```

## Performance Notes

- Incremental updates avoid storing all reads
- Sampling for duplicate detection on very large files
- K-mer counting uses HashMap with configurable k
- Per-position stats use efficient array storage
- ASCII plots generated without external dependencies