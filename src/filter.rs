use crate::{record::QualityEncoding, record::Record};
use regex::Regex;
use std::collections::HashSet;

pub struct QualityFilter {
    min_quality: f64,
    min_length: usize,
    trim_quality: Option<u8>,
    window_size: usize,
}

impl Default for QualityFilter {
    fn default() -> Self {
        QualityFilter {
            min_quality: 20.0,
            min_length: 50,
            trim_quality: Some(20),
            window_size: 4,
        }
    }
}

impl QualityFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min_quality(mut self, quality: f64) -> Self {
        self.min_quality = quality;
        self
    }

    pub fn min_length(mut self, length: usize) -> Self {
        self.min_length = length;
        self
    }

    pub fn trim_quality(mut self, quality: Option<u8>) -> Self {
        self.trim_quality = quality;
        self
    }

    pub fn window_size(mut self, size: usize) -> Self {
        self.window_size = size;
        self
    }

    pub fn filter(&self, record: &mut Record) -> bool {
        let mean_qual = record.mean_quality();

        if mean_qual < self.min_quality {
            return false;
        }

        if record.len() < self.min_length {
            return false;
        }

        true
    }

    pub fn trim<'a>(&self, record: &Record<'a>) -> Option<Record<'a>> {
        if let Some(trim_qual) = self.trim_quality {
            let (start, end) = self.sliding_window_trim(record, trim_qual);

            if end <= start {
                return None;
            }

            let trimmed_seq = &record.seq[start..end];
            let trimmed_qual = &record.qual[start..end];

            if trimmed_seq.len() < self.min_length {
                return None;
            }

            Some(Record::new(
                record.id,
                record.desc,
                trimmed_seq,
                trimmed_qual,
            ))
        } else {
            Some(Record::new(record.id, record.desc, record.seq, record.qual))
        }
    }

    fn sliding_window_trim(&self, record: &Record, quality_threshold: u8) -> (usize, usize) {
        let encoding = QualityEncoding::detect(record.qual);
        let scores = encoding.to_phred_scores(record.qual);

        let mut start = 0;
        let mut window_sum: usize = 0;

        for score in scores.iter().take(self.window_size.min(scores.len())) {
            window_sum += *score as usize;
        }

        while start + self.window_size <= scores.len() {
            let avg_quality = window_sum as f64 / self.window_size as f64;
            if avg_quality >= quality_threshold as f64 {
                break;
            }

            window_sum -= scores[start] as usize;
            if start + self.window_size < scores.len() {
                window_sum += scores[start + self.window_size] as usize;
            }
            start += 1;
        }

        let mut end = scores.len();
        window_sum = 0;

        let start_pos = end.saturating_sub(self.window_size);

        for score in scores.iter().take(end).skip(start_pos) {
            window_sum += *score as usize;
        }

        while end > start + self.window_size {
            let avg_quality = window_sum as f64 / self.window_size.min(end - start) as f64;
            if avg_quality >= quality_threshold as f64 {
                break;
            }

            end -= 1;
            if end >= self.window_size {
                window_sum -= scores[end] as usize;
                if end >= self.window_size {
                    window_sum += scores[end - self.window_size] as usize;
                }
            }
        }

        (start, end)
    }
}

pub struct AdapterTrimmer {
    adapters: Vec<Vec<u8>>,
    min_overlap: usize,
    error_rate: f64,
}

impl Default for AdapterTrimmer {
    fn default() -> Self {
        AdapterTrimmer {
            adapters: vec![b"AGATCGGAAGAGC".to_vec(), b"CTGTCTCTTATACACATCT".to_vec()],
            min_overlap: 5,
            error_rate: 0.1,
        }
    }
}

impl AdapterTrimmer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_adapter(mut self, adapter: Vec<u8>) -> Self {
        self.adapters.push(adapter);
        self
    }

    pub fn min_overlap(mut self, overlap: usize) -> Self {
        self.min_overlap = overlap;
        self
    }

    pub fn error_rate(mut self, rate: f64) -> Self {
        self.error_rate = rate;
        self
    }

    pub fn trim<'a>(&self, record: &Record<'a>) -> Record<'a> {
        let mut best_pos = record.seq.len();

        for adapter in &self.adapters {
            if let Some(pos) = self.find_adapter(record.seq, adapter) {
                if pos < best_pos {
                    best_pos = pos;
                }
            }
        }

        if best_pos < record.seq.len() {
            Record::new(
                record.id,
                record.desc,
                &record.seq[..best_pos],
                &record.qual[..best_pos],
            )
        } else {
            Record::new(record.id, record.desc, record.seq, record.qual)
        }
    }

    fn find_adapter(&self, seq: &[u8], adapter: &[u8]) -> Option<usize> {
        let max_errors = (adapter.len() as f64 * self.error_rate) as usize;

        for start in 0..=seq.len().saturating_sub(self.min_overlap) {
            let overlap_len = adapter.len().min(seq.len() - start);

            if overlap_len < self.min_overlap {
                continue;
            }

            let errors =
                self.count_mismatches(&seq[start..start + overlap_len], &adapter[..overlap_len]);

            if errors <= max_errors {
                return Some(start);
            }
        }

        None
    }

    fn count_mismatches(&self, seq1: &[u8], seq2: &[u8]) -> usize {
        seq1.iter().zip(seq2.iter()).filter(|(a, b)| a != b).count()
    }
}

#[derive(Default)]
pub struct AdvancedFilter {
    min_length: Option<usize>,
    max_length: Option<usize>,
    max_n_ratio: Option<f64>,
    max_n_count: Option<usize>,
    id_whitelist: Option<HashSet<Vec<u8>>>,
    id_blacklist: Option<HashSet<Vec<u8>>>,
    id_pattern: Option<Regex>,
}

impl AdvancedFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min_length(mut self, length: usize) -> Self {
        self.min_length = Some(length);
        self
    }

    pub fn max_length(mut self, length: usize) -> Self {
        self.max_length = Some(length);
        self
    }

    pub fn max_n_ratio(mut self, ratio: f64) -> Self {
        self.max_n_ratio = Some(ratio);
        self
    }

    pub fn max_n_count(mut self, count: usize) -> Self {
        self.max_n_count = Some(count);
        self
    }

    pub fn id_whitelist(mut self, ids: HashSet<Vec<u8>>) -> Self {
        self.id_whitelist = Some(ids);
        self
    }

    pub fn id_blacklist(mut self, ids: HashSet<Vec<u8>>) -> Self {
        self.id_blacklist = Some(ids);
        self
    }

    pub fn id_pattern(mut self, pattern: &str) -> Result<Self, regex::Error> {
        self.id_pattern = Some(Regex::new(pattern)?);
        Ok(self)
    }

    pub fn filter(&self, record: &Record) -> bool {
        if let Some(min_len) = self.min_length {
            if record.len() < min_len {
                return false;
            }
        }

        if let Some(max_len) = self.max_length {
            if record.len() > max_len {
                return false;
            }
        }

        let n_count = record
            .seq()
            .iter()
            .filter(|&&b| b == b'N' || b == b'n')
            .count();

        if let Some(max_n) = self.max_n_count {
            if n_count > max_n {
                return false;
            }
        }

        if let Some(max_ratio) = self.max_n_ratio {
            let ratio = n_count as f64 / record.len() as f64;
            if ratio > max_ratio {
                return false;
            }
        }

        if let Some(ref whitelist) = self.id_whitelist {
            if !whitelist.contains(record.id()) {
                return false;
            }
        }

        if let Some(ref blacklist) = self.id_blacklist {
            if blacklist.contains(record.id()) {
                return false;
            }
        }

        if let Some(ref pattern) = self.id_pattern {
            if let Ok(id_str) = record.id_str() {
                if !pattern.is_match(id_str) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

pub struct FilterStats {
    pub total_reads: usize,
    pub filtered_reads: usize,
    pub trimmed_reads: usize,
    pub adapter_trimmed: usize,
    pub total_bases_removed: usize,
    pub n_filtered: usize,
    pub length_filtered: usize,
    pub id_filtered: usize,
}

impl Default for FilterStats {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterStats {
    pub fn new() -> Self {
        FilterStats {
            total_reads: 0,
            filtered_reads: 0,
            trimmed_reads: 0,
            adapter_trimmed: 0,
            total_bases_removed: 0,
            n_filtered: 0,
            length_filtered: 0,
            id_filtered: 0,
        }
    }

    pub fn print_summary(&self) {
        println!("Filtering Statistics:");
        println!("  Total reads: {}", self.total_reads);
        println!("  Filtered reads: {}", self.filtered_reads);
        println!(
            "  Pass rate: {:.2}%",
            (self.filtered_reads as f64 / self.total_reads as f64) * 100.0
        );
        println!("  Trimmed reads: {}", self.trimmed_reads);
        println!("  Adapter trimmed: {}", self.adapter_trimmed);
        println!("  N-base filtered: {}", self.n_filtered);
        println!("  Length filtered: {}", self.length_filtered);
        println!("  ID filtered: {}", self.id_filtered);
        println!("  Total bases removed: {}", self.total_bases_removed);
    }
}
