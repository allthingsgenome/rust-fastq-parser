# SIMD Module

Vectorized operations for high-performance FASTQ parsing.

## Overview

The SIMD module provides AVX2-optimized functions for common parsing operations. These vectorized operations can process 32 bytes at once, providing significant performance improvements over scalar code.

## Feature Detection

The module automatically detects CPU capabilities:

```rust
pub fn has_avx2() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        is_x86_feature_detected!("avx2")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}
```

## Core Functions

### find_newlines

Find all newline positions in a buffer using SIMD.

```rust
pub fn find_newlines(data: &[u8]) -> Vec<usize>
```

**Implementation:**
```rust
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn find_newlines_avx2(data: &[u8]) -> Vec<usize> {
    use std::arch::x86_64::*;
    
    let newline = _mm256_set1_epi8(b'\n' as i8);
    let mut positions = Vec::new();
    
    for (i, chunk) in data.chunks_exact(32).enumerate() {
        let chunk_vec = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);
        let cmp = _mm256_cmpeq_epi8(chunk_vec, newline);
        let mask = _mm256_movemask_epi8(cmp);
        
        if mask != 0 {
            for j in 0..32 {
                if mask & (1 << j) != 0 {
                    positions.push(i * 32 + j);
                }
            }
        }
    }
    
    // Handle remainder with scalar code
    for (i, &byte) in data[data.len() - data.len() % 32..].iter().enumerate() {
        if byte == b'\n' {
            positions.push(data.len() - data.len() % 32 + i);
        }
    }
    
    positions
}
```

### validate_ascii

Validate that all bytes are valid ASCII characters.

```rust
pub fn validate_ascii(data: &[u8]) -> bool
```

**Example:**
```rust
let is_valid = simd::validate_ascii(b"ACGT\nACGT");
assert!(is_valid);

let invalid = simd::validate_ascii(&[0xFF, 0xFE]);
assert!(!invalid);
```

### count_chars

Count occurrences of a specific character.

```rust
pub fn count_chars(data: &[u8], target: u8) -> usize
```

**Example:**
```rust
let count = simd::count_chars(b"AAACCCGGGTTT", b'A');
assert_eq!(count, 3);
```

### find_char_positions

Find all positions of a specific character.

```rust
pub fn find_char_positions(data: &[u8], target: u8) -> Vec<usize>
```

### memchr_vectorized

Vectorized memory search for a byte.

```rust
pub fn memchr_vectorized(needle: u8, haystack: &[u8]) -> Option<usize>
```

### average_quality

Calculate average quality score using SIMD.

```rust
pub fn average_quality(quality_scores: &[u8]) -> f32
```

**Example:**
```rust
let qualities = b"IIIIIIIIIIIIIIII"; // Phred+33 scores
let avg = simd::average_quality(qualities);
```

## Nucleotide Operations

### count_nucleotides

Count all nucleotides in parallel.

```rust
pub struct NucleotideCounts {
    pub a: usize,
    pub c: usize,
    pub g: usize,
    pub t: usize,
    pub n: usize,
}

pub fn count_nucleotides(sequence: &[u8]) -> NucleotideCounts
```

**Implementation:**
```rust
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn count_nucleotides_avx2(sequence: &[u8]) -> NucleotideCounts {
    use std::arch::x86_64::*;
    
    let a_vec = _mm256_set1_epi8(b'A' as i8);
    let c_vec = _mm256_set1_epi8(b'C' as i8);
    let g_vec = _mm256_set1_epi8(b'G' as i8);
    let t_vec = _mm256_set1_epi8(b'T' as i8);
    let n_vec = _mm256_set1_epi8(b'N' as i8);
    
    let mut counts = NucleotideCounts::default();
    
    for chunk in sequence.chunks_exact(32) {
        let chunk_vec = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);
        
        counts.a += (_mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk_vec, a_vec))).count_ones() as usize;
        counts.c += (_mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk_vec, c_vec))).count_ones() as usize;
        counts.g += (_mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk_vec, g_vec))).count_ones() as usize;
        counts.t += (_mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk_vec, t_vec))).count_ones() as usize;
        counts.n += (_mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk_vec, n_vec))).count_ones() as usize;
    }
    
    // Handle remainder
    for &byte in &sequence[sequence.len() - sequence.len() % 32..] {
        match byte {
            b'A' => counts.a += 1,
            b'C' => counts.c += 1,
            b'G' => counts.g += 1,
            b'T' => counts.t += 1,
            b'N' => counts.n += 1,
            _ => {}
        }
    }
    
    counts
}
```

### gc_content

Calculate GC content percentage.

```rust
pub fn gc_content(sequence: &[u8]) -> f32
```

**Example:**
```rust
let gc = simd::gc_content(b"AACCGGTT");
assert_eq!(gc, 50.0); // 50% GC content
```

## Quality Score Operations

### quality_to_phred

Convert quality ASCII to Phred scores.

```rust
pub fn quality_to_phred(quality: &[u8], encoding: QualityEncoding) -> Vec<u8>
```

### phred_statistics

Calculate min, max, and average Phred scores.

```rust
pub struct PhredStats {
    pub min: u8,
    pub max: u8,
    pub mean: f32,
}

pub fn phred_statistics(quality: &[u8]) -> PhredStats
```

## Pattern Matching

### find_adapter

Find adapter sequences using vectorized search.

```rust
pub fn find_adapter(sequence: &[u8], adapter: &[u8]) -> Option<usize>
```

### contains_pattern

Check if sequence contains a pattern.

```rust
pub fn contains_pattern(haystack: &[u8], needle: &[u8]) -> bool
```

## Performance Utilities

### aligned_alloc

Allocate aligned memory for SIMD operations.

```rust
pub fn aligned_alloc(size: usize, alignment: usize) -> *mut u8
```

### prefetch

Prefetch data into cache.

```rust
#[inline(always)]
pub fn prefetch<T>(data: &T) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::_mm_prefetch;
        _mm_prefetch(data as *const T as *const i8, 0);
    }
}
```

## Fallback Implementations

All SIMD functions have scalar fallbacks:

```rust
fn find_newlines_scalar(data: &[u8]) -> Vec<usize> {
    data.iter()
        .enumerate()
        .filter_map(|(i, &b)| if b == b'\n' { Some(i) } else { None })
        .collect()
}

pub fn find_newlines(data: &[u8]) -> Vec<usize> {
    if has_avx2() && data.len() >= 32 {
        unsafe { find_newlines_avx2(data) }
    } else {
        find_newlines_scalar(data)
    }
}
```

## Benchmarks

Performance comparison (AVX2 vs Scalar):

| Operation | Data Size | AVX2 | Scalar | Speedup |
|-----------|-----------|------|--------|---------|
| find_newlines | 1 MB | 0.3 ms | 2.1 ms | 7.0x |
| count_chars | 1 MB | 0.2 ms | 1.8 ms | 9.0x |
| count_nucleotides | 1 MB | 0.5 ms | 3.2 ms | 6.4x |
| validate_ascii | 1 MB | 0.1 ms | 0.9 ms | 9.0x |
| gc_content | 1 MB | 0.4 ms | 2.5 ms | 6.3x |

## Examples

### Using SIMD for Record Validation

```rust
use fastq_parser::simd;

fn validate_record(record: &FastqRecord) -> bool {
    // Check sequence is valid ASCII
    if !simd::validate_ascii(&record.seq) {
        return false;
    }
    
    // Check quality scores are in valid range
    if !simd::validate_ascii(&record.qual) {
        return false;
    }
    
    // Sequence and quality must have same length
    record.seq.len() == record.qual.len()
}
```

### Fast Quality Filtering

```rust
use fastq_parser::simd;

fn filter_by_quality(records: &[FastqRecord], min_quality: f32) -> Vec<&FastqRecord> {
    records.iter()
        .filter(|record| {
            simd::average_quality(&record.qual) >= min_quality
        })
        .collect()
}
```

### Nucleotide Composition Analysis

```rust
use fastq_parser::simd;

fn analyze_composition(sequences: &[Vec<u8>]) {
    for seq in sequences {
        let counts = simd::count_nucleotides(seq);
        let gc = simd::gc_content(seq);
        
        println!("A:{} C:{} G:{} T:{} N:{} GC:{:.1}%",
                 counts.a, counts.c, counts.g, counts.t, counts.n, gc);
    }
}
```

## CPU Feature Requirements

- **AVX2**: Intel Haswell (2013) or AMD Excavator (2015) and later
- **SSE4.2**: For fallback vectorization on older CPUs
- **Scalar**: Works on all CPUs (automatic fallback)

## See Also

- [Parser Module](./parser.md) - Uses SIMD for parsing
- [Parallel Module](./parallel.md) - Combines SIMD with parallelism
- [Performance Guide](../PERFORMANCE.md) - Optimization tips