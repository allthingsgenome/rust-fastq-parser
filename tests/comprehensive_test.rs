use fastq_parser::{
    AdapterTrimmer, FastqReader, Parser, QualityEncoding, QualityFilter, Record, Result,
};

#[test]
fn test_basic_parsing() -> Result<()> {
    let data = b"@SEQ_ID\nGATTTGGGGTTCAAAGCAGTATCGATCAAATAGTAAATCCATTTGTTCAACTCACAGTTT\n+\n!''*((((***+))%%%++)(%%%%).1***-+*''))**55CCF>>>>>>CCCCCCC65\n";
    let mut parser = Parser::new(data);

    let record = parser.parse_record()?.expect("Should parse record");
    assert_eq!(record.id, b"SEQ_ID");
    assert_eq!(record.seq.len(), 60);
    assert_eq!(record.qual.len(), 60);

    Ok(())
}

#[test]
fn test_quality_encoding_detection() {
    let phred33_qual = b"!\"#$%&'()*+,-./0123456789:";
    let phred64_qual = b"@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijk";

    assert_eq!(
        QualityEncoding::detect(phred33_qual),
        QualityEncoding::Phred33
    );
    assert_eq!(
        QualityEncoding::detect(phred64_qual),
        QualityEncoding::Phred64
    );
}

#[test]
fn test_quality_score_conversion() {
    let qual_string = b"IIIIIIIIII";
    let encoding = QualityEncoding::Phred33;
    let scores = encoding.to_phred_scores(qual_string);

    assert_eq!(scores.len(), 10);
    assert!(scores.iter().all(|&s| s == 40));
}

#[test]
fn test_record_validation() -> Result<()> {
    let valid_record = Record::new(b"test", None, b"ATCG", b"IIII");
    assert!(valid_record.validate().is_ok());

    let invalid_length = Record::new(b"test", None, b"ATCG", b"III");
    assert!(invalid_length.validate().is_err());

    let invalid_base = Record::new(b"test", None, b"ATXG", b"IIII");
    assert!(invalid_base.validate().is_err());

    Ok(())
}

#[test]
fn test_quality_filter() {
    let filter = QualityFilter::new().min_quality(20.0).min_length(10);

    let good_record = Record::new(b"good", None, b"ATCGATCGATCG", b"IIIIIIIIIIII");

    let poor_quality = Record::new(b"poor", None, b"ATCGATCGATCG", b"############");

    let too_short = Record::new(b"short", None, b"ATCG", b"IIII");

    let mut good_mut = good_record.clone();
    let mut poor_mut = poor_quality.clone();
    let mut short_mut = too_short.clone();

    let good_pass = filter.filter(&mut good_mut);
    let poor_pass = filter.filter(&mut poor_mut);
    let short_pass = filter.filter(&mut short_mut);

    assert!(
        good_pass,
        "Good record should pass filter (mean quality should be ~40)"
    );
    assert!(
        !poor_pass,
        "Poor quality record should not pass filter (mean quality should be ~2)"
    );
    assert!(
        !short_pass,
        "Too short record should not pass filter (length is 4, min is 10)"
    );
}

#[test]
fn test_adapter_trimming() {
    let trimmer = AdapterTrimmer::new();

    let record_with_adapter = Record::new(
        b"test",
        None,
        b"ATCGATCGATCGAGATCGGAAGAGC",
        b"IIIIIIIIIIIIIIIIIIIIIIIII",
    );

    let trimmed = trimmer.trim(&record_with_adapter);
    assert_eq!(trimmed.seq.len(), 12);
    assert_eq!(trimmed.seq, b"ATCGATCGATCG");
}

#[test]
fn test_multiline_fastq() -> Result<()> {
    let multiline_data = b"@SEQ_ID description
ATCGATCGATCGATCG
+
IIIIIIIIIIIIIIII
";

    let mut parser = Parser::new(multiline_data);
    let record = parser.parse_record()?.expect("Should parse multiline");

    assert_eq!(record.id, b"SEQ_ID");
    assert_eq!(record.desc, Some(&b"description"[..]));
    assert!(!record.seq.is_empty());
    assert_eq!(record.seq.len(), record.qual.len());

    Ok(())
}

#[test]
fn test_gzip_support() -> Result<()> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::fs::File;
    use std::io::Write;

    let fastq_data = b"@test\nATCG\n+\nIIII\n";

    let temp_file = tempfile::Builder::new().suffix(".fastq.gz").tempfile()?;
    let path = temp_file.path().to_owned();

    {
        let file = File::create(&path)?;
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(fastq_data)?;
        encoder.finish()?;
    }

    let reader = FastqReader::from_path(&path)?;
    let records: Vec<_> = reader.into_records().collect::<Result<Vec<_>>>()?;

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, b"test");

    Ok(())
}

#[test]
fn test_mean_quality_calculation() {
    let mut record = Record::new(b"test", None, b"ATCG", b"II00");

    let mean_quality = record.mean_quality();

    let expected = (40.0 + 40.0 + 15.0 + 15.0) / 4.0;
    assert!(
        (mean_quality - expected).abs() < 0.1,
        "Expected mean quality ~{}, got {}",
        expected,
        mean_quality
    );
}
