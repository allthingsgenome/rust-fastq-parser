use fastq_parser::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;

fn main() -> Result<()> {
    println!("FASTQ Parser Advanced Features Demo\n");

    paired_end_example()?;
    filtering_example()?;
    format_conversion_example()?;
    index_example()?;
    barcode_example()?;
    metrics_example()?;

    Ok(())
}

fn paired_end_example() -> Result<()> {
    println!("=== Paired-End Read Handling ===");

    let r1_path = "testdata/sample_r1.fastq";
    let r2_path = "testdata/sample_r2.fastq";

    if Path::new(r1_path).exists() && Path::new(r2_path).exists() {
        let paired_reader = PairedEndReader::from_paths(r1_path, r2_path)?;

        let mut count = 0;
        for pair_result in paired_reader.into_paired_records().strict_pairing(true) {
            let (r1, r2) = pair_result?;
            count += 1;

            if count == 1 {
                println!("First paired read:");
                println!(
                    "  R1: {} ({}bp)",
                    String::from_utf8_lossy(&r1.id),
                    r1.seq.len()
                );
                println!(
                    "  R2: {} ({}bp)",
                    String::from_utf8_lossy(&r2.id),
                    r2.seq.len()
                );
            }
        }

        println!("Total paired reads processed: {}\n", count);
    } else {
        println!("Paired-end files not found, skipping example\n");
    }

    Ok(())
}

fn filtering_example() -> Result<()> {
    println!("=== Advanced Filtering ===");

    let filter = AdvancedFilter::new()
        .min_length(50)
        .max_length(300)
        .max_n_ratio(0.1)
        .max_n_count(10);

    let _quality_filter = QualityFilter::new().min_quality(20.0).min_length(50);

    println!("Filter settings:");
    println!("  Length: 50-300bp");
    println!("  Max N ratio: 10%");
    println!("  Max N count: 10");
    println!("  Min quality: 20");

    let test_file = "testdata/sample_clean.fastq";
    if Path::new(test_file).exists() {
        let reader = FastqReader::from_path(test_file)?;
        let mut passed = 0;
        let mut failed = 0;

        for record_result in reader.into_records() {
            let record = record_result?;
            let record_ref = record.as_record();

            if filter.filter(&record_ref) {
                passed += 1;
            } else {
                failed += 1;
            }
        }

        println!("Filtering results:");
        println!("  Passed: {}", passed);
        println!("  Failed: {}", failed);
        println!(
            "  Pass rate: {:.2}%\n",
            (passed as f64 / (passed + failed) as f64) * 100.0
        );
    } else {
        println!("Test file not found, skipping example\n");
    }

    Ok(())
}

fn format_conversion_example() -> Result<()> {
    println!("=== Format Conversion ===");

    let input_file = "testdata/sample_clean.fastq";
    let output_file = "output/sample.fasta";

    if Path::new(input_file).exists() {
        std::fs::create_dir_all("output").ok();

        let count = FormatConverter::fastq_to_fasta(input_file, output_file)?;
        println!("Converted {} reads from FASTQ to FASTA", count);
        println!("Output written to: {}\n", output_file);

        let filter_fn = |record: &Record| record.len() >= 100;
        let filtered_output = "output/filtered.fastq";
        let (total, passed) =
            FormatConverter::filter_and_write(input_file, filtered_output, filter_fn)?;

        println!("Filtered writing:");
        println!("  Total reads: {}", total);
        println!("  Passed filter: {}", passed);
        println!("  Output: {}\n", filtered_output);
    } else {
        println!("Input file not found, skipping example\n");
    }

    Ok(())
}

fn index_example() -> Result<()> {
    println!("=== Index-Based Random Access ===");

    let fastq_file = "testdata/sample_clean.fastq";
    let index_file = "output/sample.fqi";

    if Path::new(fastq_file).exists() {
        std::fs::create_dir_all("output").ok();

        println!("Building index...");
        let index = FastqIndex::build(fastq_file)?;
        println!("Index built with {} entries", index.len());

        index.save(index_file)?;
        println!("Index saved to: {}", index_file);

        let reader = IndexedReader::new(fastq_file, index)?;

        let ids: Vec<String> = reader.index().ids().take(3).cloned().collect();
        if !ids.is_empty() {
            println!("\nRandom access examples:");
            for id in &ids {
                if let Some(record) = reader.get_record(id) {
                    println!("  Retrieved {}: {}bp", id, record.seq().len());
                }
            }
        }

        println!();
    } else {
        println!("FASTQ file not found, skipping example\n");
    }

    Ok(())
}

fn barcode_example() -> Result<()> {
    println!("=== Barcode/UMI Processing ===");

    let config = BarcodeConfig::new(0, 8).with_umi(8, 10).max_mismatches(1);

    println!("Barcode configuration:");
    println!("  Barcode: positions 0-8");
    println!("  UMI: positions 8-18");
    println!("  Max mismatches: 1");

    let mut barcodes = HashMap::new();
    barcodes.insert(b"ATCGATCG".to_vec(), "Sample1".to_string());
    barcodes.insert(b"GCTAGCTA".to_vec(), "Sample2".to_string());
    barcodes.insert(b"TTAACCGG".to_vec(), "Sample3".to_string());

    let demux = Demultiplexer::new(config.clone(), barcodes).error_correction(true);

    println!("\nDemultiplexing simulation:");
    let test_barcodes = vec![b"ATCGATCG", b"GCTAGCTA", b"ATCGATGG", b"TTTTTTTT"];

    for barcode in test_barcodes {
        let sample = demux.assign_sample(barcode);
        println!(
            "  {} -> {:?}",
            String::from_utf8_lossy(barcode),
            sample.unwrap_or("Undetermined".to_string())
        );
    }

    let mut known = HashSet::new();
    known.insert(b"ATCGATCG".to_vec());
    known.insert(b"GCTAGCTA".to_vec());

    let corrector = BarcodeCorrector::new(known, 1);

    println!("\nBarcode correction:");
    let test_corrections = vec![b"ATCGATGG", b"GCTAGCTT", b"TTTTTTTT"];

    for barcode in test_corrections {
        let corrected = corrector.correct(barcode);
        println!(
            "  {} -> {:?}",
            String::from_utf8_lossy(barcode),
            corrected.map(|c| String::from_utf8_lossy(&c).into_owned())
        );
    }

    println!();
    Ok(())
}

fn metrics_example() -> Result<()> {
    println!("=== Advanced Quality Metrics ===");

    let test_file = "testdata/sample_clean.fastq";

    if Path::new(test_file).exists() {
        let reader = FastqReader::from_path(test_file)?;
        let mut metrics = QualityMetrics::new();

        println!("Calculating metrics...");
        for record_result in reader.into_records() {
            let record = record_result?;
            let mut record_ref = record.as_record();
            metrics.update(&mut record_ref);
        }

        metrics.finalize();

        let summary = metrics.summary();
        println!("\nQuality Metrics Summary:");
        println!("  Total reads: {}", summary.total_reads);
        println!("  Total bases: {}", summary.total_bases);
        println!(
            "  Read length range: {}-{}",
            summary.min_length, summary.max_length
        );
        println!("  Mean read length: {:.1}", summary.mean_length);
        println!("  Mean GC content: {:.2}%", summary.mean_gc);
        println!("  N-base percentage: {:.4}%", summary.n_base_percent);
        println!("  Duplicate rate: {:.2}%", summary.duplicate_rate * 100.0);

        let pos_stats = metrics.position_quality_stats();
        if pos_stats.len() > 0 {
            println!("\nPer-position quality (first 5 positions):");
            for stat in pos_stats.iter().take(5) {
                println!(
                    "  Position {}: mean={:.1}, median={}, Q25={}, Q75={}",
                    stat.position + 1,
                    stat.mean,
                    stat.median,
                    stat.q25,
                    stat.q75
                );
            }
        }

        println!("\nASCII Quality Plot:");
        let plot = QualityPlotter::generate_ascii_plot(&pos_stats, 60, 15);
        print!("{}", plot);
    } else {
        println!("Test file not found, skipping example\n");
    }

    Ok(())
}
