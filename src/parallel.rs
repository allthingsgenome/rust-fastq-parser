use crate::{
    error::Result,
    filter::QualityFilter,
    parser::Parser,
    record::{OwnedRecord, Record},
};
use crossbeam_channel::{bounded, Sender};
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

const CHUNK_SIZE: usize = 1024 * 1024;
const QUEUE_SIZE: usize = 100;

pub struct ParallelParser {
    data: Arc<Vec<u8>>,
    num_threads: usize,
}

impl ParallelParser {
    pub fn new(data: Vec<u8>) -> Self {
        let num_threads = rayon::current_num_threads();
        ParallelParser {
            data: Arc::new(data),
            num_threads,
        }
    }

    pub fn with_threads(data: Vec<u8>, num_threads: usize) -> Self {
        ParallelParser {
            data: Arc::new(data),
            num_threads,
        }
    }

    pub fn parse(&self) -> Result<Vec<OwnedRecord>> {
        let chunks = self.find_record_boundaries();

        chunks
            .par_iter()
            .map(|&(start, end)| {
                let slice = &self.data[start..end];
                let parser = Parser::new(slice);
                let mut records = Vec::new();

                for record in parser {
                    records.push(OwnedRecord::from_record(&record));
                }

                Ok(records)
            })
            .try_fold(Vec::new, |mut acc, chunk_result| {
                chunk_result.map(|chunk| {
                    acc.extend(chunk);
                    acc
                })
            })
            .try_reduce(Vec::new, |mut acc, chunk| {
                acc.extend(chunk);
                Ok(acc)
            })
    }

    pub fn parse_with_callback<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(OwnedRecord) + Send + Sync,
    {
        let chunks = self.find_record_boundaries();

        chunks.par_iter().try_for_each(|&(start, end)| {
            let slice = &self.data[start..end];
            let parser = Parser::new(slice);

            for record in parser {
                callback(OwnedRecord::from_record(&record));
            }

            Ok(())
        })
    }

    pub fn parse_streaming(&self) -> crossbeam_channel::Receiver<Result<OwnedRecord>> {
        let (sender, receiver) = bounded(1000);
        let data = Arc::clone(&self.data);
        let chunks = self.find_record_boundaries();

        thread::spawn(move || {
            chunks.par_iter().for_each(|&(start, end)| {
                let slice = &data[start..end];
                let parser = Parser::new(slice);

                for record in parser {
                    let owned = OwnedRecord::from_record(&record);
                    if sender.send(Ok(owned)).is_err() {
                        break;
                    }
                }
            });
        });

        receiver
    }

    fn find_record_boundaries(&self) -> Vec<(usize, usize)> {
        let mut boundaries = Vec::new();
        let data = &*self.data;
        let len = data.len();

        if len == 0 {
            return boundaries;
        }

        let chunk_size = (len / self.num_threads).max(CHUNK_SIZE);
        let mut start = 0;

        while start < len {
            let mut end = (start + chunk_size).min(len);

            if end < len {
                // Use SIMD to find the next record boundary
                while end < len {
                    // Look for @ after a newline
                    if let Some(at_pos) = crate::simd::find_char(data, b'@', end) {
                        // Check if there's a newline before it
                        if at_pos > 0 && data[at_pos - 1] == b'\n' {
                            end = at_pos;
                            break;
                        }
                        end = at_pos + 1;
                    } else {
                        end = len;
                        break;
                    }
                }
                boundaries.push((start, end));
                start = end;
            } else {
                boundaries.push((start, len));
                break;
            }
        }

        boundaries
    }
}

pub struct ChunkedProcessor {
    chunk_size: usize,
    buffer_size: usize,
}

impl Default for ChunkedProcessor {
    fn default() -> Self {
        ChunkedProcessor {
            chunk_size: 4 * 1024 * 1024,
            buffer_size: 1000,
        }
    }
}

impl ChunkedProcessor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn process<F>(&self, data: &[u8], processor: F) -> Result<()>
    where
        F: Fn(&OwnedRecord) -> Result<()> + Send + Sync + 'static,
    {
        let (sender, receiver) = bounded(self.buffer_size);
        let processor = Arc::new(processor);

        let handle = thread::spawn({
            let processor = Arc::clone(&processor);
            move || {
                while let Ok(record) = receiver.recv() {
                    if let Err(e) = processor(&record) {
                        eprintln!("Error processing record: {}", e);
                    }
                }
            }
        });

        self.parse_chunks(data, sender)?;

        handle.join().unwrap();
        Ok(())
    }

    fn parse_chunks(&self, data: &[u8], sender: Sender<OwnedRecord>) -> Result<()> {
        let mut pos = 0;

        while pos < data.len() {
            let end = self.find_chunk_end(data, pos);
            let chunk = &data[pos..end];

            let parser = Parser::new(chunk);
            for record in parser {
                let owned = OwnedRecord::from_record(&record);
                if sender.send(owned).is_err() {
                    break;
                }
            }

            pos = end;
        }

        Ok(())
    }

    fn find_chunk_end(&self, data: &[u8], start: usize) -> usize {
        let mut end = (start + self.chunk_size).min(data.len());

        if end >= data.len() {
            return data.len();
        }

        while end < data.len() && data[end] != b'@' {
            end += 1;
        }

        if end < data.len() && end > 0 && data[end - 1] == b'\n' {
            return end;
        }

        while end < data.len() && data[end] != b'\n' {
            end += 1;
        }

        if end < data.len() {
            end + 1
        } else {
            data.len()
        }
    }
}

pub struct ParallelProcessor<F> {
    processor: Arc<F>,
    num_threads: usize,
    chunk_size: usize,
    progress: Arc<AtomicUsize>,
}

impl<F> ParallelProcessor<F>
where
    F: Fn(OwnedRecord) -> Result<()> + Send + Sync + 'static,
{
    pub fn new(processor: F) -> Self {
        ParallelProcessor {
            processor: Arc::new(processor),
            num_threads: rayon::current_num_threads(),
            chunk_size: CHUNK_SIZE,
            progress: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn with_threads(processor: F, num_threads: usize) -> Self {
        ParallelProcessor {
            processor: Arc::new(processor),
            num_threads,
            chunk_size: CHUNK_SIZE,
            progress: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn process_file(&self, data: &[u8]) -> Result<ProcessingStats> {
        let (sender, receiver) = bounded(QUEUE_SIZE);
        let processor = Arc::clone(&self.processor);
        let progress = Arc::clone(&self.progress);

        let stats = Arc::new(Mutex::new(ProcessingStats::new()));
        let stats_clone = Arc::clone(&stats);

        let workers: Vec<_> = (0..self.num_threads)
            .map(|_| {
                let receiver = receiver.clone();
                let processor = Arc::clone(&processor);
                let progress = Arc::clone(&progress);
                let stats = Arc::clone(&stats_clone);

                thread::spawn(move || {
                    while let Ok(record) = receiver.recv() {
                        match processor(record) {
                            Ok(_) => {
                                progress.fetch_add(1, Ordering::Relaxed);
                                stats.lock().unwrap().processed += 1;
                            }
                            Err(_) => {
                                stats.lock().unwrap().failed += 1;
                            }
                        }
                    }
                })
            })
            .collect();

        self.parse_and_send(data, sender)?;

        for worker in workers {
            worker.join().unwrap();
        }

        let final_stats = stats.lock().unwrap().clone();
        Ok(final_stats)
    }

    fn parse_and_send(&self, data: &[u8], sender: Sender<OwnedRecord>) -> Result<()> {
        let chunks = self.split_into_chunks(data);

        chunks.par_iter().try_for_each(|&(start, end)| {
            let slice = &data[start..end];
            let parser = Parser::new(slice);

            for record in parser {
                let owned = OwnedRecord::from_record(&record);
                sender.send(owned).map_err(|_| {
                    crate::error::FastqError::Io(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "Channel closed",
                    ))
                })?;
            }

            Ok(())
        })
    }

    fn split_into_chunks(&self, data: &[u8]) -> Vec<(usize, usize)> {
        let mut chunks = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            let end = self.find_chunk_boundary(data, pos, self.chunk_size);
            chunks.push((pos, end));
            pos = end;
        }

        chunks
    }

    fn find_chunk_boundary(&self, data: &[u8], start: usize, target_size: usize) -> usize {
        let mut end = (start + target_size).min(data.len());

        if end >= data.len() {
            return data.len();
        }

        // Use SIMD to find record boundary
        while end < data.len() {
            if let Some(at_pos) = crate::simd::find_char(data, b'@', end) {
                if at_pos > 0 && data[at_pos - 1] == b'\n' {
                    return at_pos;
                }
                end = at_pos + 1;
            } else {
                break;
            }
        }

        data.len()
    }

    pub fn get_progress(&self) -> usize {
        self.progress.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone)]
pub struct ProcessingStats {
    pub processed: usize,
    pub failed: usize,
    pub total_bases: usize,
    pub total_quality: f64,
}

impl Default for ProcessingStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessingStats {
    pub fn new() -> Self {
        ProcessingStats {
            processed: 0,
            failed: 0,
            total_bases: 0,
            total_quality: 0.0,
        }
    }

    pub fn print_summary(&self) {
        println!("Processing Statistics:");
        println!("  Processed: {} records", self.processed);
        println!("  Failed: {} records", self.failed);
        println!(
            "  Success rate: {:.2}%",
            (self.processed as f64 / (self.processed + self.failed) as f64) * 100.0
        );
        if self.processed > 0 {
            println!(
                "  Average quality: {:.2}",
                self.total_quality / self.processed as f64
            );
        }
    }
}

pub struct ParallelFilterProcessor {
    filter: Arc<QualityFilter>,
    num_workers: usize,
}

impl ParallelFilterProcessor {
    pub fn new(filter: QualityFilter) -> Self {
        ParallelFilterProcessor {
            filter: Arc::new(filter),
            num_workers: rayon::current_num_threads(),
        }
    }

    pub fn process<R, W>(&self, input: R, output: W) -> Result<ProcessingStats>
    where
        R: std::io::Read + Send + 'static,
        W: std::io::Write + Send + 'static,
    {
        let (input_sender, input_receiver) = bounded::<OwnedRecord>(QUEUE_SIZE);
        let (output_sender, output_receiver) = bounded::<OwnedRecord>(QUEUE_SIZE);

        let filter = Arc::clone(&self.filter);
        let stats = Arc::new(Mutex::new(ProcessingStats::new()));

        let reader_thread = thread::spawn(move || {
            let reader = crate::stream::StreamingReader::new(input);
            for result in reader {
                match result {
                    Ok(record) => {
                        if input_sender.send(record).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let filter_workers: Vec<_> = (0..self.num_workers)
            .map(|_| {
                let input_rx = input_receiver.clone();
                let output_tx = output_sender.clone();
                let filter = Arc::clone(&filter);
                let stats = Arc::clone(&stats);

                thread::spawn(move || {
                    while let Ok(record) = input_rx.recv() {
                        let record_ref = record.as_record();
                        let mut record_mut = Record::new(
                            record_ref.id,
                            record_ref.desc,
                            record_ref.seq,
                            record_ref.qual,
                        );

                        if filter.filter(&mut record_mut) {
                            if let Some(trimmed) = filter.trim(&record_mut) {
                                let owned = OwnedRecord::from_record(&trimmed);
                                if output_tx.send(owned).is_err() {
                                    break;
                                }
                                stats.lock().unwrap().processed += 1;
                            }
                        } else {
                            stats.lock().unwrap().failed += 1;
                        }
                    }
                })
            })
            .collect();

        drop(input_receiver);
        drop(output_sender);

        let writer_thread = thread::spawn(move || {
            let mut output = output;
            while let Ok(record) = output_receiver.recv() {
                let record_ref = record.as_record();
                let formatted = format!("{}", record_ref);
                if output.write_all(formatted.as_bytes()).is_err() {
                    break;
                }
            }
        });

        reader_thread.join().unwrap();
        for worker in filter_workers {
            worker.join().unwrap();
        }
        writer_thread.join().unwrap();

        let final_stats = stats.lock().unwrap().clone();
        Ok(final_stats)
    }
}
