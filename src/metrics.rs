use crate::record::Record;
use std::collections::{HashMap, HashSet};

pub struct QualityMetrics {
    position_qualities: Vec<Vec<u8>>,
    duplicate_tracker: DuplicateTracker,
    kmer_counter: KmerCounter,
    total_reads: usize,
    total_bases: usize,
    gc_content: Vec<f64>,
    n_bases: usize,
    min_length: usize,
    max_length: usize,
    mean_length: f64,
}

impl Default for QualityMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl QualityMetrics {
    pub fn new() -> Self {
        QualityMetrics {
            position_qualities: Vec::new(),
            duplicate_tracker: DuplicateTracker::new(),
            kmer_counter: KmerCounter::new(5),
            total_reads: 0,
            total_bases: 0,
            gc_content: Vec::new(),
            n_bases: 0,
            min_length: usize::MAX,
            max_length: 0,
            mean_length: 0.0,
        }
    }

    pub fn update(&mut self, record: &mut Record) {
        self.total_reads += 1;
        self.total_bases += record.len();

        if record.len() < self.min_length {
            self.min_length = record.len();
        }
        if record.len() > self.max_length {
            self.max_length = record.len();
        }

        while self.position_qualities.len() < record.len() {
            self.position_qualities.push(Vec::new());
        }

        let phred_scores = record.phred_scores();
        for (pos, &score) in phred_scores.iter().enumerate() {
            self.position_qualities[pos].push(score);
        }

        let gc_count = record
            .seq()
            .iter()
            .filter(|&&b| b == b'G' || b == b'C' || b == b'g' || b == b'c')
            .count();
        let gc_percent = (gc_count as f64 / record.len() as f64) * 100.0;
        self.gc_content.push(gc_percent);

        self.n_bases += record
            .seq()
            .iter()
            .filter(|&&b| b == b'N' || b == b'n')
            .count();

        self.duplicate_tracker.add(record.seq());

        self.kmer_counter.count_kmers(record.seq());
    }

    pub fn finalize(&mut self) {
        self.mean_length = self.total_bases as f64 / self.total_reads as f64;
    }

    pub fn position_quality_stats(&self) -> Vec<PositionStats> {
        self.position_qualities
            .iter()
            .enumerate()
            .map(|(pos, qualities)| {
                if qualities.is_empty() {
                    PositionStats {
                        position: pos,
                        mean: 0.0,
                        median: 0,
                        q25: 0,
                        q75: 0,
                        min: 0,
                        max: 0,
                    }
                } else {
                    let mut sorted = qualities.clone();
                    sorted.sort_unstable();

                    let mean = sorted.iter().map(|&q| q as f64).sum::<f64>() / sorted.len() as f64;
                    let median = sorted[sorted.len() / 2];
                    let q25 = sorted[sorted.len() / 4];
                    let q75 = sorted[sorted.len() * 3 / 4];
                    let min = *sorted.first().unwrap();
                    let max = *sorted.last().unwrap();

                    PositionStats {
                        position: pos,
                        mean,
                        median,
                        q25,
                        q75,
                        min,
                        max,
                    }
                }
            })
            .collect()
    }

    pub fn duplicate_rate(&self) -> f64 {
        self.duplicate_tracker.duplicate_rate()
    }

    pub fn exact_duplicates(&self) -> usize {
        self.duplicate_tracker.exact_duplicates()
    }

    pub fn kmer_distribution(&self) -> &HashMap<Vec<u8>, usize> {
        self.kmer_counter.distribution()
    }

    pub fn error_kmers(&self, threshold: f64) -> Vec<Vec<u8>> {
        self.kmer_counter.error_kmers(self.total_reads, threshold)
    }

    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            total_reads: self.total_reads,
            total_bases: self.total_bases,
            min_length: self.min_length,
            max_length: self.max_length,
            mean_length: self.mean_length,
            mean_gc: self.gc_content.iter().sum::<f64>() / self.gc_content.len() as f64,
            n_base_percent: (self.n_bases as f64 / self.total_bases as f64) * 100.0,
            duplicate_rate: self.duplicate_rate(),
        }
    }

    pub fn print_summary(&self) {
        let summary = self.summary();
        println!("Quality Metrics Summary:");
        println!("  Total reads: {}", summary.total_reads);
        println!("  Total bases: {}", summary.total_bases);
        println!(
            "  Read length: {} - {} (mean: {:.1})",
            summary.min_length, summary.max_length, summary.mean_length
        );
        println!("  GC content: {:.2}%", summary.mean_gc);
        println!("  N-base percentage: {:.4}%", summary.n_base_percent);
        println!("  Duplicate rate: {:.2}%", summary.duplicate_rate * 100.0);
    }
}

#[derive(Debug, Clone)]
pub struct PositionStats {
    pub position: usize,
    pub mean: f64,
    pub median: u8,
    pub q25: u8,
    pub q75: u8,
    pub min: u8,
    pub max: u8,
}

#[derive(Debug, Clone)]
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

struct DuplicateTracker {
    seen_sequences: HashSet<Vec<u8>>,
    duplicate_count: usize,
    total_count: usize,
    use_sampling: bool,
    sample_size: usize,
}

impl DuplicateTracker {
    fn new() -> Self {
        DuplicateTracker {
            seen_sequences: HashSet::new(),
            duplicate_count: 0,
            total_count: 0,
            use_sampling: false,
            sample_size: 100000,
        }
    }

    fn add(&mut self, seq: &[u8]) {
        self.total_count += 1;

        if self.use_sampling && self.total_count > self.sample_size {
            return;
        }

        if self.seen_sequences.len() > 1000000 && !self.use_sampling {
            self.use_sampling = true;
            self.seen_sequences.clear();
            return;
        }

        if !self.seen_sequences.insert(seq.to_vec()) {
            self.duplicate_count += 1;
        }
    }

    fn duplicate_rate(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.duplicate_count as f64 / self.total_count as f64
        }
    }

    fn exact_duplicates(&self) -> usize {
        self.duplicate_count
    }
}

struct KmerCounter {
    k: usize,
    counts: HashMap<Vec<u8>, usize>,
}

impl KmerCounter {
    fn new(k: usize) -> Self {
        KmerCounter {
            k,
            counts: HashMap::new(),
        }
    }

    fn count_kmers(&mut self, seq: &[u8]) {
        if seq.len() < self.k {
            return;
        }

        for window in seq.windows(self.k) {
            *self.counts.entry(window.to_vec()).or_insert(0) += 1;
        }
    }

    fn distribution(&self) -> &HashMap<Vec<u8>, usize> {
        &self.counts
    }

    fn error_kmers(&self, total_reads: usize, threshold: f64) -> Vec<Vec<u8>> {
        let min_count = (total_reads as f64 * threshold) as usize;

        self.counts
            .iter()
            .filter(|(_, &count)| count < min_count)
            .map(|(kmer, _)| kmer.clone())
            .collect()
    }
}

pub struct ErrorDetector {
    kmer_size: usize,
    min_frequency: usize,
    error_threshold: f64,
}

impl Default for ErrorDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorDetector {
    pub fn new() -> Self {
        ErrorDetector {
            kmer_size: 5,
            min_frequency: 10,
            error_threshold: 0.001,
        }
    }

    pub fn kmer_size(mut self, size: usize) -> Self {
        self.kmer_size = size;
        self
    }

    pub fn min_frequency(mut self, freq: usize) -> Self {
        self.min_frequency = freq;
        self
    }

    pub fn error_threshold(mut self, threshold: f64) -> Self {
        self.error_threshold = threshold;
        self
    }

    pub fn detect_errors(
        &self,
        seq: &[u8],
        kmer_counts: &HashMap<Vec<u8>, usize>,
    ) -> Vec<ErrorPosition> {
        let mut errors = Vec::new();

        if seq.len() < self.kmer_size {
            return errors;
        }

        for (pos, window) in seq.windows(self.kmer_size).enumerate() {
            let count = kmer_counts.get(window).copied().unwrap_or(0);

            if count < self.min_frequency {
                let mut max_neighbor_count = 0;
                let mut likely_correction = None;

                for i in 0..self.kmer_size {
                    for base in b"ACGT" {
                        if window[i] != *base {
                            let mut neighbor = window.to_vec();
                            neighbor[i] = *base;

                            if let Some(&neighbor_count) = kmer_counts.get(&neighbor) {
                                if neighbor_count > max_neighbor_count {
                                    max_neighbor_count = neighbor_count;
                                    likely_correction = Some((pos + i, *base));
                                }
                            }
                        }
                    }
                }

                if max_neighbor_count >= self.min_frequency {
                    if let Some((error_pos, correct_base)) = likely_correction {
                        errors.push(ErrorPosition {
                            position: error_pos,
                            incorrect_base: seq[error_pos],
                            suggested_base: correct_base,
                            confidence: (max_neighbor_count as f64) / ((count + 1) as f64),
                        });
                    }
                }
            }
        }

        errors
    }
}

#[derive(Debug, Clone)]
pub struct ErrorPosition {
    pub position: usize,
    pub incorrect_base: u8,
    pub suggested_base: u8,
    pub confidence: f64,
}

pub struct QualityPlotter;

impl QualityPlotter {
    pub fn generate_ascii_plot(stats: &[PositionStats], width: usize, height: usize) -> String {
        if stats.is_empty() {
            return String::from("No data to plot");
        }

        let max_quality = stats.iter().map(|s| s.max).max().unwrap_or(40) as f64;
        let min_quality = stats.iter().map(|s| s.min).min().unwrap_or(0) as f64;
        let quality_range = max_quality - min_quality;

        let mut plot = vec![vec![' '; width]; height];

        for (y, row) in plot.iter_mut().enumerate().take(height) {
            let quality = max_quality - (y as f64 * quality_range / (height - 1) as f64);
            let label = format!("{:3.0}", quality);
            for (i, ch) in label.chars().enumerate() {
                if i < 3 {
                    row[i] = ch;
                }
            }
            row[4] = '|';
        }

        for x in 5..width {
            plot[height - 1][x] = '-';
        }

        let positions_per_column = stats.len().max(1) / (width - 6).max(1);

        for (col, chunk) in stats.chunks(positions_per_column.max(1)).enumerate() {
            if col + 6 >= width {
                break;
            }

            let mean_quality = chunk.iter().map(|s| s.mean).sum::<f64>() / chunk.len() as f64;
            let median_quality = chunk[chunk.len() / 2].median as f64;

            let mean_y =
                ((max_quality - mean_quality) * (height - 1) as f64 / quality_range) as usize;
            let median_y =
                ((max_quality - median_quality) * (height - 1) as f64 / quality_range) as usize;

            if mean_y < height {
                plot[mean_y][col + 6] = '*';
            }
            if median_y < height && median_y != mean_y {
                plot[median_y][col + 6] = 'o';
            }
        }

        let mut result = String::new();
        result.push_str("Quality Score Distribution (* = mean, o = median)\n");
        for row in plot {
            result.push_str(&row.iter().collect::<String>());
            result.push('\n');
        }
        result.push_str("    Position in read →\n");

        result
    }
}
