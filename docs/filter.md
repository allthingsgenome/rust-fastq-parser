# Filter Module

The `filter` module provides comprehensive filtering capabilities for FASTQ reads based on quality, length, content, and adapters.

## Types

### `QualityFilter`

Basic quality-based filtering.

```rust
pub struct QualityFilter {
    min_quality: f64,
    min_length: usize,
    trim_quality: Option<u8>,
    window_size: usize,
}
```

#### Methods

- `min_quality(q)` - Set minimum mean quality score
- `min_length(len)` - Set minimum read length
- `trim_quality(q)` - Enable quality trimming
- `window_size(w)` - Set sliding window size for trimming
- `filter(&record)` - Check if record passes filter
- `trim(&record)` - Trim low-quality ends

### `AdvancedFilter`

Extended filtering with multiple criteria.

```rust
pub struct AdvancedFilter {
    min_length: Option<usize>,
    max_length: Option<usize>,
    max_n_ratio: Option<f64>,
    max_n_count: Option<usize>,
    id_whitelist: Option<HashSet<Vec<u8>>>,
    id_blacklist: Option<HashSet<Vec<u8>>>,
    id_pattern: Option<Regex>,
}
```

#### Methods

- `min_length(len)` - Set minimum length
- `max_length(len)` - Set maximum length
- `max_n_ratio(ratio)` - Maximum ratio of N bases (0.0-1.0)
- `max_n_count(count)` - Maximum absolute N count
- `id_whitelist(ids)` - Only keep specific IDs
- `id_blacklist(ids)` - Exclude specific IDs
- `id_pattern(regex)` - Match ID pattern
- `filter(&record)` - Apply all filters

### `AdapterTrimmer`

Remove adapter sequences from reads.

```rust
pub struct AdapterTrimmer {
    adapters: Vec<Vec<u8>>,
    min_overlap: usize,
    error_rate: f64,
}
```

#### Methods

- `add_adapter(seq)` - Add adapter sequence
- `min_overlap(len)` - Minimum overlap for detection
- `error_rate(rate)` - Allowed mismatch rate
- `trim(&record)` - Remove adapters from record

### `FilterStats`

Track filtering statistics.

```rust
pub struct FilterStats {
    pub total_reads: usize,
    pub filtered_reads: usize,
    pub trimmed_reads: usize,
    pub adapter_trimmed: usize,
    pub n_filtered: usize,
    pub length_filtered: usize,
    pub id_filtered: usize,
    pub total_bases_removed: usize,
}
```

## Usage Examples

### Basic Quality Filtering

```rust
use fastq_parser::QualityFilter;

let filter = QualityFilter::new()
    .min_quality(20.0)
    .min_length(50);

for record in records {
    if filter.filter(&mut record) {
        // Process passing reads
    }
}
```

### Quality Trimming

```rust
let filter = QualityFilter::new()
    .trim_quality(Some(20))  // Trim bases below Q20
    .window_size(4);          // 4bp sliding window

if let Some(trimmed) = filter.trim(&record) {
    // Use trimmed record
    println!("Trimmed to {}bp", trimmed.len());
}
```

### Advanced Content Filtering

```rust
use fastq_parser::AdvancedFilter;

let filter = AdvancedFilter::new()
    .min_length(50)
    .max_length(300)
    .max_n_ratio(0.1)     // Max 10% N bases
    .max_n_count(5);      // Max 5 N bases total

let mut stats = FilterStats::new();

for record in records {
    stats.total_reads += 1;
    if filter.filter(&record) {
        stats.filtered_reads += 1;
        // Process passing read
    }
}

stats.print_summary();
```

### ID-Based Filtering

```rust
use std::collections::HashSet;

// Whitelist specific reads
let mut whitelist = HashSet::new();
whitelist.insert(b"READ1".to_vec());
whitelist.insert(b"READ2".to_vec());

let filter = AdvancedFilter::new()
    .id_whitelist(whitelist);

// Or use regex pattern
let filter = AdvancedFilter::new()
    .id_pattern("^SAMPLE1_.*")?;  // Only SAMPLE1 reads
```

### Adapter Trimming

```rust
use fastq_parser::AdapterTrimmer;

let trimmer = AdapterTrimmer::new()
    .add_adapter(b"AGATCGGAAGAGC".to_vec())      // Illumina
    .add_adapter(b"CTGTCTCTTATACACATCT".to_vec()) // Nextera
    .min_overlap(5)
    .error_rate(0.1);  // 10% mismatches allowed

let trimmed = trimmer.trim(&record);
if trimmed.len() < record.len() {
    println!("Removed {} bases", record.len() - trimmed.len());
}
```

### Combined Filtering Pipeline

```rust
use fastq_parser::{QualityFilter, AdapterTrimmer, AdvancedFilter};

// Setup filters
let quality_filter = QualityFilter::new().min_quality(20.0);
let adapter_trimmer = AdapterTrimmer::new();
let content_filter = AdvancedFilter::new().max_n_ratio(0.1);

let mut stats = FilterStats::new();

for record in records {
    stats.total_reads += 1;
    
    // Trim adapters
    let trimmed = adapter_trimmer.trim(&record);
    if trimmed.len() < record.len() {
        stats.adapter_trimmed += 1;
    }
    
    // Quality filter
    if !quality_filter.filter(&mut trimmed) {
        continue;
    }
    
    // Content filter
    if !content_filter.filter(&trimmed) {
        continue;
    }
    
    stats.filtered_reads += 1;
    // Process passing read
}
```

## Filter Statistics

```rust
let stats = FilterStats::new();
// ... apply filters and update stats ...

stats.print_summary();
// Output:
// Filtering Statistics:
//   Total reads: 10000
//   Filtered reads: 8500
//   Pass rate: 85.00%
//   Trimmed reads: 2000
//   Adapter trimmed: 1500
//   N-base filtered: 200
//   Length filtered: 300
//   Total bases removed: 50000
```

## Performance Notes

- Sliding window trimming uses efficient incremental computation
- Adapter matching supports mismatches via Hamming distance
- Regex patterns are compiled once and reused
- HashSet lookups for ID filtering are O(1)