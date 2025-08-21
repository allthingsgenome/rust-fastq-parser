use fastq_parser::*;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_advanced_filter() {
    let data = b"@SEQ1\nACGTNNACGT\n+\nIIIIIIIIII\n@SEQ2\nACGTACGTAC\n+\nIIIIIIIIII\n";
    let parser = Parser::new(data);
    
    let filter = AdvancedFilter::new()
        .min_length(5)
        .max_length(15)
        .max_n_count(1);
    
    let records: Vec<_> = parser.collect();
    let mut filtered = Vec::new();
    
    for record in records {
        if filter.filter(&record) {
            filtered.push(record);
        }
    }
    
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id(), b"SEQ2");
}

#[test]
fn test_paired_end_reader() {
    let r1_data = b"@READ1/1\nACGTACGT\n+\nIIIIIIII\n@READ2/1\nTGCATGCA\n+\nIIIIIIII\n";
    let r2_data = b"@READ1/2\nGGGGAAAA\n+\nIIIIIIII\n@READ2/2\nCCCCTTTT\n+\nIIIIIIII\n";
    
    let mut r1_file = NamedTempFile::new().unwrap();
    let mut r2_file = NamedTempFile::new().unwrap();
    
    r1_file.write_all(r1_data).unwrap();
    r2_file.write_all(r2_data).unwrap();
    
    let paired_reader = PairedEndReader::from_paths(r1_file.path(), r2_file.path()).unwrap();
    let pairs: Vec<_> = paired_reader.into_paired_records().collect();
    
    assert_eq!(pairs.len(), 2);
    
    for pair in pairs {
        let (r1, r2) = pair.unwrap();
        assert!(r1.id.starts_with(b"READ"));
        assert!(r2.id.starts_with(b"READ"));
    }
}

#[test]
fn test_format_conversion() {
    let fastq_data = b"@SEQ1 description\nACGTACGT\n+\nIIIIIIII\n";
    let mut fastq_file = NamedTempFile::new().unwrap();
    let fasta_file = NamedTempFile::new().unwrap();
    
    fastq_file.write_all(fastq_data).unwrap();
    
    let count = FormatConverter::fastq_to_fasta(fastq_file.path(), fasta_file.path()).unwrap();
    assert_eq!(count, 1);
    
    let fasta_content = std::fs::read_to_string(fasta_file.path()).unwrap();
    assert!(fasta_content.contains(">SEQ1 description"));
    assert!(fasta_content.contains("ACGTACGT"));
}

#[test]
fn test_fastq_writer() {
    let record = Record::new(b"TEST", Some(b"desc"), b"ACGT", b"IIII");
    let mut buffer = Vec::new();
    
    {
        let mut writer = FastqWriter::new(&mut buffer);
        writer.write_record(&record).unwrap();
    }
    
    let written = String::from_utf8(buffer).unwrap();
    assert_eq!(written, "@TEST desc\nACGT\n+\nIIII\n");
}

#[test]
fn test_barcode_extraction() {
    let config = BarcodeConfig::new(0, 8)
        .with_umi(8, 10);
    
    let extractor = BarcodeExtractor::new(config);
    let record = Record::new(
        b"READ1",
        None,
        b"ATCGATCGATCGATCGATCGATCG",
        b"IIIIIIIIIIIIIIIIIIIIIIII"
    );
    
    let (barcode, umi) = extractor.extract(&record).unwrap();
    assert_eq!(barcode, b"ATCGATCG");
    assert_eq!(umi, Some(b"ATCGATCGAT".to_vec()));
}

#[test]
fn test_demultiplexing() {
    let mut barcodes = HashMap::new();
    barcodes.insert(b"ATCGATCG".to_vec(), "Sample1".to_string());
    barcodes.insert(b"GCTAGCTA".to_vec(), "Sample2".to_string());
    
    let config = BarcodeConfig::new(0, 8);
    let demux = Demultiplexer::new(config, barcodes);
    
    assert_eq!(demux.assign_sample(b"ATCGATCG"), Some("Sample1".to_string()));
    assert_eq!(demux.assign_sample(b"GCTAGCTA"), Some("Sample2".to_string()));
    assert_eq!(demux.assign_sample(b"TTTTTTTT"), None);
}

#[test]
fn test_quality_metrics() {
    let data = b"@SEQ1\nACGTACGT\n+\nIIIIJJJJ\n@SEQ2\nGCGCGCGC\n+\nKKKKLLLL\n";
    let parser = Parser::new(data);
    
    let mut metrics = QualityMetrics::new();
    
    for mut record in parser {
        metrics.update(&mut record);
    }
    
    metrics.finalize();
    
    let summary = metrics.summary();
    assert_eq!(summary.total_reads, 2);
    assert_eq!(summary.total_bases, 16);
    assert_eq!(summary.min_length, 8);
    assert_eq!(summary.max_length, 8);
}

#[test]
fn test_fastq_index() {
    let data = b"@READ1\nACGT\n+\nIIII\n@READ2\nTGCA\n+\nJJJJ\n";
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(data).unwrap();
    
    let index = FastqIndex::build(file.path()).unwrap();
    assert_eq!(index.len(), 2);
    assert!(index.contains("READ1"));
    assert!(index.contains("READ2"));
    
    let entry = index.get("READ1").unwrap();
    assert_eq!(entry.seq_length, 4);
}

#[test]
fn test_indexed_reader() {
    let data = b"@READ1\nACGT\n+\nIIII\n@READ2\nTGCA\n+\nJJJJ\n";
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(data).unwrap();
    
    let index = FastqIndex::build(file.path()).unwrap();
    let reader = IndexedReader::new(file.path(), index).unwrap();
    
    let record1 = reader.get_record("READ1").unwrap();
    assert_eq!(record1.seq(), b"ACGT");
    
    let record2 = reader.get_record("READ2").unwrap();
    assert_eq!(record2.seq(), b"TGCA");
    
    let batch = reader.get_batch(&["READ1", "READ2", "NOTFOUND"]);
    assert_eq!(batch.len(), 3);
    assert!(batch[0].is_some());
    assert!(batch[1].is_some());
    assert!(batch[2].is_none());
}

#[test]
fn test_umi_deduplication() {
    let records = vec![
        OwnedRecord {
            id: b"READ1:UMI_AAA_BC_GGG".to_vec(),
            desc: None,
            seq: b"ACGT".to_vec(),
            qual: b"IIII".to_vec(),
        },
        OwnedRecord {
            id: b"READ2:UMI_AAA_BC_GGG".to_vec(),
            desc: None,
            seq: b"ACGT".to_vec(),
            qual: b"JJJJ".to_vec(),
        },
        OwnedRecord {
            id: b"READ3:UMI_BBB_BC_GGG".to_vec(),
            desc: None,
            seq: b"ACGT".to_vec(),
            qual: b"KKKK".to_vec(),
        },
    ];
    
    let dedup = UmiDeduplicator::new();
    let deduplicated = dedup.deduplicate(records.into_iter());
    
    assert_eq!(deduplicated.len(), 2);
}

#[test]
fn test_barcode_correction() {
    let mut known = HashSet::new();
    known.insert(b"ATCGATCG".to_vec());
    known.insert(b"GCTAGCTA".to_vec());
    
    let corrector = BarcodeCorrector::new(known, 1);
    
    assert_eq!(corrector.correct(b"ATCGATCG"), Some(b"ATCGATCG".to_vec()));
    assert_eq!(corrector.correct(b"ATCGATGG"), Some(b"ATCGATCG".to_vec()));
    assert_eq!(corrector.correct(b"TTTTTTTT"), None);
}