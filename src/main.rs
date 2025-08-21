use fastq_parser::{
    parallel::{ParallelFilterProcessor, ParallelProcessor},
    AdapterTrimmer, FastqReader, FilterStats, QualityEncoding, QualityFilter, Result,
};
use std::fs::File;
use std::io::{self, Write};
use std::time::Instant;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <input.fastq[.gz]> [options]", args[0]);
        eprintln!("\nOptions:");
        eprintln!("  --filter        Apply quality filtering");
        eprintln!("  --trim          Trim low quality bases");
        eprintln!("  --parallel      Use parallel processing");
        eprintln!("  --stats         Print statistics");
        eprintln!("  --output <file> Write filtered reads to file");
        eprintln!("\nExamples:");
        eprintln!("  {} input.fastq --stats", args[0]);
        eprintln!(
            "  {} input.fastq.gz --filter --trim --output filtered.fastq",
            args[0]
        );
        eprintln!("  {} input.fastq --parallel --filter", args[0]);
        return Ok(());
    }

    let input_path = &args[1];
    let do_filter = args.contains(&"--filter".to_string());
    let do_trim = args.contains(&"--trim".to_string());
    let do_parallel = args.contains(&"--parallel".to_string());
    let do_stats = args.contains(&"--stats".to_string());
    let output_path = args
        .iter()
        .position(|arg| arg == "--output")
        .and_then(|i| args.get(i + 1));

    let start = Instant::now();

    if do_parallel && (do_filter || do_trim) {
        process_parallel_filter(input_path, output_path)?;
    } else if do_parallel {
        process_parallel(input_path)?;
    } else if do_filter || do_trim {
        process_with_filter(input_path, output_path, do_filter, do_trim)?;
    } else if do_stats {
        print_statistics(input_path)?;
    } else {
        process_simple(input_path)?;
    }

    let elapsed = start.elapsed();
    println!("\nProcessing time: {:.3} seconds", elapsed.as_secs_f64());

    Ok(())
}

fn process_simple(input_path: &str) -> Result<()> {
    println!("Processing FASTQ file: {}", input_path);

    let reader = FastqReader::from_path(input_path)?;
    let mut count = 0;
    let mut total_length = 0;

    for result in reader.into_records() {
        let record = result?;
        count += 1;
        total_length += record.seq.len();

        if count <= 5 {
            let record_ref = record.as_record();
            println!(
                "\nRecord {}: {}",
                count,
                std::str::from_utf8(record_ref.id).unwrap_or("<invalid>")
            );
            println!("  Sequence length: {}", record_ref.seq.len());

            let mut rec_mut = record.as_record();
            let encoding = rec_mut.quality_encoding();
            println!("  Quality encoding: {:?}", encoding);
            println!("  Mean quality: {:.2}", rec_mut.mean_quality());
        }

        if count % 100_000 == 0 {
            println!("Processed {} records...", count);
        }
    }

    println!("\nTotal records: {}", count);
    println!("Total bases: {}", total_length);
    println!(
        "Average read length: {:.2}",
        total_length as f64 / count as f64
    );

    Ok(())
}

fn process_with_filter(
    input_path: &str,
    output_path: Option<&String>,
    do_filter: bool,
    do_trim: bool,
) -> Result<()> {
    println!("Processing with filtering: {}", input_path);

    let filter = QualityFilter::new()
        .min_quality(20.0)
        .min_length(50)
        .trim_quality(if do_trim { Some(20) } else { None });

    let adapter_trimmer = AdapterTrimmer::new();

    let reader = FastqReader::from_path(input_path)?;
    let mut stats = FilterStats::new();

    let mut output: Box<dyn Write> = if let Some(path) = output_path {
        Box::new(File::create(path)?)
    } else {
        Box::new(io::stdout())
    };

    for result in reader.into_records() {
        let record = result?;
        stats.total_reads += 1;

        let mut record_ref = record.as_record();

        if !do_filter || filter.filter(&mut record_ref) {
            let trimmed = if do_trim {
                adapter_trimmer.trim(&record_ref)
            } else {
                record_ref
            };

            if let Some(quality_trimmed) = filter.trim(&trimmed) {
                stats.filtered_reads += 1;

                if quality_trimmed.len() < trimmed.len() {
                    stats.trimmed_reads += 1;
                    stats.total_bases_removed += trimmed.len() - quality_trimmed.len();
                }

                if output_path.is_some() {
                    writeln!(output, "{}", quality_trimmed)?;
                }
            }
        }

        if stats.total_reads % 100_000 == 0 {
            eprintln!("Processed {} records...", stats.total_reads);
        }
    }

    if output_path.is_none() {
        eprintln!();
    }

    stats.print_summary();

    Ok(())
}

fn process_parallel(input_path: &str) -> Result<()> {
    println!("Processing in parallel: {}", input_path);

    let data = std::fs::read(input_path)?;

    let processor = ParallelProcessor::new(|record| {
        let _seq_len = record.seq.len();
        Ok(())
    });

    let stats = processor.process_file(&data)?;
    stats.print_summary();

    Ok(())
}

fn process_parallel_filter(input_path: &str, output_path: Option<&String>) -> Result<()> {
    println!("Processing with parallel filtering: {}", input_path);

    let filter = QualityFilter::new()
        .min_quality(20.0)
        .min_length(50)
        .trim_quality(Some(20));

    let processor = ParallelFilterProcessor::new(filter);

    let input = File::open(input_path)?;
    let output: Box<dyn Write + Send> = if let Some(path) = output_path {
        Box::new(File::create(path)?)
    } else {
        Box::new(io::stdout())
    };

    let stats = processor.process(input, output)?;

    if output_path.is_none() {
        eprintln!();
    }

    stats.print_summary();

    Ok(())
}

fn print_statistics(input_path: &str) -> Result<()> {
    println!("Analyzing FASTQ file: {}", input_path);

    let reader = FastqReader::from_path(input_path)?;

    let mut total_records = 0;
    let mut total_bases = 0;
    let mut min_length = usize::MAX;
    let mut max_length = 0;
    let mut total_quality = 0.0;
    let mut quality_encoding = None;
    let mut gc_count = 0;

    for result in reader.into_records() {
        let record = result?;
        let record_ref = record.as_record();

        total_records += 1;
        let seq_len = record_ref.seq.len();
        total_bases += seq_len;

        min_length = min_length.min(seq_len);
        max_length = max_length.max(seq_len);

        for &base in record_ref.seq {
            if base == b'G' || base == b'C' || base == b'g' || base == b'c' {
                gc_count += 1;
            }
        }

        if quality_encoding.is_none() {
            quality_encoding = Some(QualityEncoding::detect(record_ref.qual));
        }

        let mut rec_mut = record.as_record();
        total_quality += rec_mut.mean_quality();

        if total_records % 100_000 == 0 {
            println!("Analyzed {} records...", total_records);
        }
    }

    println!("\nStatistics:");
    println!("  Total records: {}", total_records);
    println!("  Total bases: {}", total_bases);
    println!("  Min read length: {}", min_length);
    println!("  Max read length: {}", max_length);
    println!(
        "  Average read length: {:.2}",
        total_bases as f64 / total_records as f64
    );
    println!(
        "  GC content: {:.2}%",
        (gc_count as f64 / total_bases as f64) * 100.0
    );
    println!(
        "  Quality encoding: {:?}",
        quality_encoding.unwrap_or(QualityEncoding::Unknown)
    );
    println!(
        "  Average quality score: {:.2}",
        total_quality / total_records as f64
    );

    Ok(())
}
