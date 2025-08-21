use crate::{error::Result, record::{Record, OwnedRecord}, writer::FastqWriter};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;

type ExtractedBarcode = Option<(Vec<u8>, Option<Vec<u8>>)>;

#[derive(Debug, Clone)]
pub struct BarcodeConfig {
    pub barcode_start: usize,
    pub barcode_length: usize,
    pub umi_start: Option<usize>,
    pub umi_length: Option<usize>,
    pub max_mismatches: usize,
    pub in_header: bool,
}

impl Default for BarcodeConfig {
    fn default() -> Self {
        BarcodeConfig {
            barcode_start: 0,
            barcode_length: 8,
            umi_start: None,
            umi_length: None,
            max_mismatches: 1,
            in_header: false,
        }
    }
}

impl BarcodeConfig {
    pub fn new(barcode_start: usize, barcode_length: usize) -> Self {
        BarcodeConfig {
            barcode_start,
            barcode_length,
            ..Default::default()
        }
    }
    
    pub fn with_umi(mut self, start: usize, length: usize) -> Self {
        self.umi_start = Some(start);
        self.umi_length = Some(length);
        self
    }
    
    pub fn max_mismatches(mut self, mismatches: usize) -> Self {
        self.max_mismatches = mismatches;
        self
    }
    
    pub fn in_header(mut self, in_header: bool) -> Self {
        self.in_header = in_header;
        self
    }
}

pub struct BarcodeExtractor {
    config: BarcodeConfig,
}

impl BarcodeExtractor {
    pub fn new(config: BarcodeConfig) -> Self {
        BarcodeExtractor { config }
    }
    
    pub fn extract(&self, record: &Record) -> Option<(Vec<u8>, Option<Vec<u8>>)> {
        let source = if self.config.in_header {
            record.id()
        } else {
            record.seq()
        };
        
        if source.len() < self.config.barcode_start + self.config.barcode_length {
            return None;
        }
        
        let barcode = source[self.config.barcode_start..self.config.barcode_start + self.config.barcode_length].to_vec();
        
        let umi = if let (Some(umi_start), Some(umi_length)) = (self.config.umi_start, self.config.umi_length) {
            if source.len() >= umi_start + umi_length {
                Some(source[umi_start..umi_start + umi_length].to_vec())
            } else {
                None
            }
        } else {
            None
        };
        
        Some((barcode, umi))
    }
    
    pub fn extract_and_trim<'a>(&self, record: &'a Record<'a>) -> (ExtractedBarcode, Record<'a>) {
        if self.config.in_header {
            (self.extract(record), Record::new(record.id(), record.desc(), record.seq(), record.qual()))
        } else {
            let extracted = self.extract(record);
            
            if extracted.is_some() {
                let mut trim_end = self.config.barcode_start + self.config.barcode_length;
                
                if let (Some(umi_start), Some(umi_length)) = (self.config.umi_start, self.config.umi_length) {
                    if umi_start + umi_length > trim_end {
                        trim_end = umi_start + umi_length;
                    }
                }
                
                let trimmed_seq = &record.seq()[trim_end..];
                let trimmed_qual = &record.qual()[trim_end..];
                
                (extracted, Record::new(record.id(), record.desc(), trimmed_seq, trimmed_qual))
            } else {
                (None, Record::new(record.id(), record.desc(), record.seq(), record.qual()))
            }
        }
    }
}

pub struct Demultiplexer {
    config: BarcodeConfig,
    barcodes: HashMap<Vec<u8>, String>,
    error_correction: bool,
}

impl Demultiplexer {
    pub fn new(config: BarcodeConfig, barcodes: HashMap<Vec<u8>, String>) -> Self {
        Demultiplexer {
            config,
            barcodes,
            error_correction: true,
        }
    }
    
    pub fn error_correction(mut self, enabled: bool) -> Self {
        self.error_correction = enabled;
        self
    }
    
    pub fn assign_sample(&self, barcode: &[u8]) -> Option<String> {
        if let Some(sample) = self.barcodes.get(barcode) {
            return Some(sample.clone());
        }
        
        if self.error_correction && self.config.max_mismatches > 0 {
            let mut best_match = None;
            let mut best_distance = self.config.max_mismatches + 1;
            
            for (known_barcode, sample) in &self.barcodes {
                let distance = hamming_distance(barcode, known_barcode);
                if distance <= self.config.max_mismatches && distance < best_distance {
                    best_distance = distance;
                    best_match = Some(sample.clone());
                }
            }
            
            best_match
        } else {
            None
        }
    }
    
    pub fn demultiplex_to_files<P: AsRef<Path>, I>(
        &self,
        records: I,
        output_dir: P,
        prefix: &str,
    ) -> Result<DemultiplexStats>
    where
        I: Iterator<Item = Result<OwnedRecord>>,
    {
        use std::fs;
        
        let output_dir = output_dir.as_ref();
        fs::create_dir_all(output_dir)?;
        
        let mut writers: HashMap<String, FastqWriter<File>> = HashMap::new();
        let mut undetermined_writer = FastqWriter::to_file(output_dir.join(format!("{}_undetermined.fastq", prefix)))?;
        
        let mut stats = DemultiplexStats::new();
        let extractor = BarcodeExtractor::new(self.config.clone());
        
        for record_result in records {
            let record = record_result?;
            let record_ref = record.as_record();
            stats.total_reads += 1;
            
            let (extracted, trimmed_record) = extractor.extract_and_trim(&record_ref);
            
            if let Some((barcode, umi)) = extracted {
                if let Some(sample) = self.assign_sample(&barcode) {
                    stats.assigned_reads += 1;
                    *stats.sample_counts.entry(sample.clone()).or_insert(0) += 1;
                    
                    if !writers.contains_key(&sample) {
                        let output_path = output_dir.join(format!("{}_{}.fastq", prefix, sample));
                        writers.insert(sample.clone(), FastqWriter::to_file(output_path)?);
                    }
                    
                    let writer = writers.get_mut(&sample).unwrap();
                    
                    let mut modified_record = OwnedRecord::from_record(&trimmed_record);
                    if let Some(umi) = umi {
                        let umi_str = String::from_utf8_lossy(&umi);
                        let barcode_str = String::from_utf8_lossy(&barcode);
                        let new_id = format!("{}:UMI_{}_BC_{}", 
                                            String::from_utf8_lossy(&modified_record.id),
                                            umi_str,
                                            barcode_str);
                        modified_record.id = new_id.into_bytes();
                    }
                    
                    writer.write_owned_record(&modified_record)?;
                } else {
                    stats.undetermined_reads += 1;
                    undetermined_writer.write_record(&trimmed_record)?;
                }
            } else {
                stats.no_barcode_reads += 1;
                undetermined_writer.write_owned_record(&record)?;
            }
        }
        
        for writer in writers.values_mut() {
            writer.flush()?;
        }
        undetermined_writer.flush()?;
        
        Ok(stats)
    }
}

pub struct DemultiplexStats {
    pub total_reads: usize,
    pub assigned_reads: usize,
    pub undetermined_reads: usize,
    pub no_barcode_reads: usize,
    pub sample_counts: HashMap<String, usize>,
}

impl Default for DemultiplexStats {
    fn default() -> Self {
        Self::new()
    }
}

impl DemultiplexStats {
    pub fn new() -> Self {
        DemultiplexStats {
            total_reads: 0,
            assigned_reads: 0,
            undetermined_reads: 0,
            no_barcode_reads: 0,
            sample_counts: HashMap::new(),
        }
    }
    
    pub fn print_summary(&self) {
        println!("Demultiplexing Statistics:");
        println!("  Total reads: {}", self.total_reads);
        println!("  Assigned reads: {} ({:.2}%)", 
                 self.assigned_reads,
                 (self.assigned_reads as f64 / self.total_reads as f64) * 100.0);
        println!("  Undetermined reads: {} ({:.2}%)",
                 self.undetermined_reads,
                 (self.undetermined_reads as f64 / self.total_reads as f64) * 100.0);
        println!("  No barcode reads: {}", self.no_barcode_reads);
        println!("\nSample distribution:");
        
        let mut sorted_samples: Vec<_> = self.sample_counts.iter().collect();
        sorted_samples.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        
        for (sample, count) in sorted_samples {
            println!("  {}: {} ({:.2}%)",
                     sample,
                     count,
                     (*count as f64 / self.total_reads as f64) * 100.0);
        }
    }
}

pub struct UmiDeduplicator {
    min_quality: Option<f64>,
}

impl Default for UmiDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

impl UmiDeduplicator {
    pub fn new() -> Self {
        UmiDeduplicator {
            min_quality: None,
        }
    }
    
    pub fn min_quality(mut self, quality: f64) -> Self {
        self.min_quality = Some(quality);
        self
    }
    
    pub fn deduplicate<I>(&self, records: I) -> Vec<OwnedRecord>
    where
        I: Iterator<Item = OwnedRecord>,
    {
        let mut seen_umis: HashMap<(Vec<u8>, Vec<u8>), OwnedRecord> = HashMap::new();
        
        for record in records {
            let umi = self.extract_umi_from_header(&record);
            if let Some(umi) = umi {
                let key = (umi, record.seq.clone());
                
                match seen_umis.get(&key) {
                    Some(existing) => {
                        if self.should_replace(existing, &record) {
                            seen_umis.insert(key, record);
                        }
                    }
                    None => {
                        seen_umis.insert(key, record);
                    }
                }
            } else {
                seen_umis.insert((vec![], record.seq.clone()), record);
            }
        }
        
        seen_umis.into_values().collect()
    }
    
    fn extract_umi_from_header(&self, record: &OwnedRecord) -> Option<Vec<u8>> {
        let id_str = String::from_utf8_lossy(&record.id);
        if let Some(umi_pos) = id_str.find(":UMI_") {
            let umi_start = umi_pos + 5;
            let umi_end = id_str[umi_start..].find('_').map(|p| umi_start + p)
                .unwrap_or(id_str.len());
            Some(id_str[umi_start..umi_end].as_bytes().to_vec())
        } else {
            None
        }
    }
    
    fn should_replace(&self, existing: &OwnedRecord, new: &OwnedRecord) -> bool {
        if let Some(min_qual) = self.min_quality {
            let mut existing_record = existing.as_record();
            let mut new_record = new.as_record();
            let existing_quality = existing_record.mean_quality();
            let new_quality = new_record.mean_quality();
            new_quality > existing_quality && new_quality >= min_qual
        } else {
            false
        }
    }
}

fn hamming_distance(s1: &[u8], s2: &[u8]) -> usize {
    if s1.len() != s2.len() {
        return usize::MAX;
    }
    
    s1.iter().zip(s2.iter()).filter(|(a, b)| a != b).count()
}

pub struct BarcodeCorrector {
    known_barcodes: HashSet<Vec<u8>>,
    max_distance: usize,
}

impl BarcodeCorrector {
    pub fn new(known_barcodes: HashSet<Vec<u8>>, max_distance: usize) -> Self {
        BarcodeCorrector {
            known_barcodes,
            max_distance,
        }
    }
    
    pub fn correct(&self, barcode: &[u8]) -> Option<Vec<u8>> {
        if self.known_barcodes.contains(barcode) {
            return Some(barcode.to_vec());
        }
        
        let mut best_match = None;
        let mut best_distance = self.max_distance + 1;
        
        for known in &self.known_barcodes {
            let distance = hamming_distance(barcode, known);
            if distance <= self.max_distance && distance < best_distance {
                best_distance = distance;
                best_match = Some(known.clone());
            }
        }
        
        best_match
    }
}