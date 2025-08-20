use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use fastq_parser::{Parser, FastqReader, parallel::ParallelParser};
use std::io::Write;
use tempfile::NamedTempFile;

fn generate_fastq_data(num_records: usize, seq_len: usize) -> Vec<u8> {
    let mut data = Vec::new();
    
    for i in 0..num_records {
        writeln!(data, "@SEQ_{} description", i).unwrap();
        
        for _ in 0..seq_len {
            data.push(b"ACGT"[fastq_parser::simd::bytecount::count(&[i as u8], 0) % 4]);
        }
        data.push(b'\n');
        
        writeln!(data, "+").unwrap();
        
        for _ in 0..seq_len {
            data.push(b'I');
        }
        data.push(b'\n');
    }
    
    data
}

fn bench_basic_parser(c: &mut Criterion) {
    let data = generate_fastq_data(10000, 150);
    let mut group = c.benchmark_group("basic_parser");
    group.throughput(Throughput::Bytes(data.len() as u64));
    
    group.bench_function("parse_10k_records", |b| {
        b.iter(|| {
            let parser = Parser::new(&data);
            let count = parser.count();
            black_box(count);
        });
    });
    
    group.finish();
}

fn bench_parallel_parser(c: &mut Criterion) {
    let data = generate_fastq_data(10000, 150);
    let mut group = c.benchmark_group("parallel_parser");
    group.throughput(Throughput::Bytes(data.len() as u64));
    
    group.bench_function("parallel_parse_10k", |b| {
        // Pre-allocate data outside the benchmark loop
        let data_vec = data.clone();
        b.iter(|| {
            let parser = ParallelParser::new(data_vec.clone());
            let records = parser.parse().unwrap();
            black_box(records.len());
        });
    });
    
    group.finish();
}

fn bench_mmap_reader(c: &mut Criterion) {
    let data = generate_fastq_data(10000, 150);
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(&data).unwrap();
    
    let mut group = c.benchmark_group("mmap_reader");
    group.throughput(Throughput::Bytes(data.len() as u64));
    
    group.bench_function("mmap_10k_records", |b| {
        b.iter(|| {
            let reader = FastqReader::from_file(temp_file.path()).unwrap();
            let count = reader.into_records().count();
            black_box(count);
        });
    });
    
    group.finish();
}

fn bench_simd_operations(c: &mut Criterion) {
    let data = generate_fastq_data(1000, 150);
    let mut group = c.benchmark_group("simd");
    
    group.bench_function("find_newlines", |b| {
        b.iter(|| {
            let positions = fastq_parser::simd::find_newlines(&data);
            black_box(positions);
        });
    });
    
    group.bench_function("validate_ascii", |b| {
        b.iter(|| {
            let valid = fastq_parser::simd::validate_ascii(&data);
            black_box(valid);
        });
    });
    
    group.bench_function("count_chars", |b| {
        b.iter(|| {
            let count = fastq_parser::simd::count_chars(&data, b'A');
            black_box(count);
        });
    });
    
    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let sizes = vec![1000, 10000, 100000];
    let mut group = c.benchmark_group("memory_scaling");
    
    for size in sizes {
        let data = generate_fastq_data(size, 150);
        group.throughput(Throughput::Elements(size as u64));
        
        group.bench_function(format!("parse_{}_records", size), |b| {
            b.iter(|| {
                let parser = Parser::new(&data);
                let mut count = 0;
                for _ in parser {
                    count += 1;
                }
                black_box(count);
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_basic_parser,
    bench_parallel_parser,
    bench_mmap_reader,
    bench_simd_operations,
    bench_memory_usage
);
criterion_main!(benches);