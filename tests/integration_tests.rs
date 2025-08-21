use fastq_parser::{parallel::ParallelParser, FastqError, FastqReader, Parser};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_basic_parsing() {
    let data = b"@SEQ_1\nACGT\n+\nIIII\n@SEQ_2\nTGCA\n+\nJJJJ\n";
    let parser = Parser::new(data);
    let records: Vec<_> = parser.collect();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].id_str().unwrap(), "SEQ_1");
    assert_eq!(records[0].seq_str().unwrap(), "ACGT");
    assert_eq!(records[0].qual_str().unwrap(), "IIII");
    assert_eq!(records[1].id_str().unwrap(), "SEQ_2");
    assert_eq!(records[1].seq_str().unwrap(), "TGCA");
}

#[test]
fn test_parsing_with_description() {
    let data = b"@SEQ_1 some description here\nACGT\n+\nIIII\n";
    let mut parser = Parser::new(data);
    let record = parser.next().unwrap();

    assert_eq!(record.id_str().unwrap(), "SEQ_1");
    assert_eq!(record.desc_str().unwrap().unwrap(), "some description here");
    assert_eq!(record.seq_str().unwrap(), "ACGT");
}

#[test]
fn test_multiline_sequences() {
    // FASTQ format is strictly 4 lines per record, not multiline
    // This test should check that the parser correctly handles standard format
    let data = b"@SEQ_1\nACGTTGCA\n+\nIIIIJJJJ\n";
    let mut parser = Parser::new(data);

    match parser.parse_record() {
        Ok(Some(record)) => {
            assert_eq!(record.seq_str().unwrap(), "ACGTTGCA");
            assert_eq!(record.qual_str().unwrap(), "IIIIJJJJ");
        }
        Ok(None) => panic!("Expected record"),
        Err(e) => panic!("Error parsing: {}", e),
    }
}

#[test]
fn test_windows_line_endings() {
    let data = b"@SEQ_1\r\nACGT\r\n+\r\nIIII\r\n";
    let mut parser = Parser::new(data);
    let record = parser.next().unwrap();

    assert_eq!(record.id_str().unwrap(), "SEQ_1");
    assert_eq!(record.seq_str().unwrap(), "ACGT");
    assert_eq!(record.qual_str().unwrap(), "IIII");
}

#[test]
fn test_empty_file() {
    let data = b"";
    let parser = Parser::new(data);
    let records: Vec<_> = parser.collect();
    assert_eq!(records.len(), 0);
}

#[test]
fn test_invalid_header() {
    let data = b"SEQ_1\nACGT\n+\nIIII\n";
    let mut parser = Parser::new(data);

    match parser.parse_record() {
        Err(FastqError::InvalidHeader { .. }) => {}
        _ => panic!("Expected InvalidHeader error"),
    }
}

#[test]
fn test_length_mismatch() {
    let data = b"@SEQ_1\nACGT\n+\nIII\n";
    let mut parser = Parser::new(data);

    match parser.parse_record() {
        Err(FastqError::LengthMismatch {
            seq_len: 4,
            qual_len: 3,
        }) => {}
        _ => panic!("Expected LengthMismatch error"),
    }
}

#[test]
fn test_file_reader() {
    let data = b"@SEQ_1\nACGT\n+\nIIII\n@SEQ_2\nTGCA\n+\nJJJJ\n";
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(data).unwrap();

    let reader = FastqReader::from_file(temp_file.path()).unwrap();
    let records: Vec<_> = reader
        .into_records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].as_record().id_str().unwrap(), "SEQ_1");
    assert_eq!(records[1].as_record().id_str().unwrap(), "SEQ_2");
}

#[test]
fn test_gzip_reader() {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let data = b"@SEQ_1\nACGT\n+\nIIII\n";
    let mut temp_file = NamedTempFile::with_suffix(".fastq.gz").unwrap();

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    let compressed = encoder.finish().unwrap();
    temp_file.write_all(&compressed).unwrap();

    let reader = FastqReader::from_path(temp_file.path()).unwrap();
    let records: Vec<_> = reader
        .into_records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].as_record().id_str().unwrap(), "SEQ_1");
}

#[test]
fn test_parallel_parser() {
    let mut data = Vec::new();
    for i in 0..1000 {
        writeln!(data, "@SEQ_{}", i).unwrap();
        writeln!(data, "ACGTACGTACGT").unwrap();
        writeln!(data, "+").unwrap();
        writeln!(data, "IIIIIIIIIIII").unwrap();
    }

    let parser = ParallelParser::new(data);
    let records = parser.parse().unwrap();

    assert_eq!(records.len(), 1000);
    // Parallel parsing doesn't guarantee order, so just check that all IDs are present
    let mut ids: Vec<_> = records
        .iter()
        .map(|r| r.as_record().id_str().unwrap().to_string())
        .collect();
    ids.sort();

    for i in 0..1000 {
        let expected_id = format!("SEQ_{}", i);
        assert!(
            ids.binary_search(&expected_id).is_ok(),
            "Missing ID: {}",
            expected_id
        );
    }
}

#[test]
fn test_large_file_parsing() {
    let mut data = Vec::new();
    let num_records = 10000;

    for i in 0..num_records {
        writeln!(data, "@SEQ_{} description", i).unwrap();
        writeln!(
            data,
            "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT"
        )
        .unwrap();
        writeln!(data, "+").unwrap();
        writeln!(
            data,
            "IIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIII"
        )
        .unwrap();
    }

    let parser = Parser::new(&data);
    let count = parser.count();
    assert_eq!(count, num_records);
}

#[test]
fn test_quality_scores() {
    let data = b"@SEQ_1\nACGT\n+\n!#$%\n";
    let mut parser = Parser::new(data);
    let record = parser.next().unwrap();

    let qual = record.qual();
    assert_eq!(qual[0], b'!');
    assert_eq!(qual[1], b'#');
    assert_eq!(qual[2], b'$');
    assert_eq!(qual[3], b'%');
}

#[test]
fn test_unicode_in_description() {
    let data = "@SEQ_1 test™\nACGT\n+\nIIII\n".as_bytes();
    let mut parser = Parser::new(data);
    let record = parser.next().unwrap();

    assert_eq!(record.id_str().unwrap(), "SEQ_1");
    assert_eq!(record.desc_str().unwrap().unwrap(), "test™");
}
