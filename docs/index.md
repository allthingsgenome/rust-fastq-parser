# Index Module

The `index` module provides index-based random access to FASTQ files, enabling O(1) lookup of specific reads.

## Types

### `FastqIndex`

Persistent index structure for FASTQ files.

```rust
pub struct FastqIndex {
    entries: HashMap<String, IndexEntry>,
    total_records: usize,
    file_size: u64,
}
```

#### Methods

- `build(path)` - Build index from FASTQ file
- `save(path)` - Save index to file (binary format)
- `load(path)` - Load index from file
- `get(id)` - Get index entry for read ID
- `contains(id)` - Check if ID exists
- `len()` - Total number of records
- `ids()` - Iterator over all read IDs

### `IndexedReader`

Memory-mapped reader using an index for random access.

```rust
pub struct IndexedReader {
    mmap: Mmap,
    index: FastqIndex,
}
```

#### Methods

- `new(fastq_path, index)` - Create with index
- `from_paths(fastq_path, index_path)` - Load from files
- `get_record(id)` - Get single record by ID (O(1))
- `get_owned_record(id)` - Get owned record by ID
- `get_batch(ids)` - Get multiple records
- `iter_range(start, count)` - Iterate over range

### `RandomAccessReader`

File-based reader with seeking (no memory mapping).

#### Methods

- `new(fastq_path, index)` - Create with index
- `get_record(id)` - Get record by seeking to position

## Usage Examples

### Building an Index

```rust
use fastq_parser::FastqIndex;

// Build index from FASTQ file
let index = FastqIndex::build("reads.fastq")?;
println!("Indexed {} reads", index.len());

// Save for reuse
index.save("reads.fqi")?;
```

### Random Access Lookup

```rust
use fastq_parser::IndexedReader;

// Load index and create reader
let reader = IndexedReader::from_paths("reads.fastq", "reads.fqi")?;

// O(1) lookup by ID
if let Some(record) = reader.get_record("READ_12345") {
    println!("Found: {} ({}bp)", 
             record.id_str()?, record.len());
}

// Batch retrieval
let ids = vec!["READ_1", "READ_2", "READ_3"];
let records = reader.get_batch(&ids);
```

### Range Iteration

```rust
// Iterate over specific range
for record in reader.iter_range(1000, 100) {
    // Process reads 1000-1099
    println!("ID: {}", String::from_utf8_lossy(&record.id));
}
```

### Check Index Contents

```rust
let index = FastqIndex::load("reads.fqi")?;

// Check if read exists
if index.contains("READ_12345") {
    let entry = index.get("READ_12345").unwrap();
    println!("Read at offset {}, length {}bp", 
             entry.offset, entry.seq_length);
}

// List all IDs
for id in index.ids().take(10) {
    println!("ID: {}", id);
}
```

## Index Format

The index uses binary serialization (bincode) containing:
- HashMap of read ID → file offset, record length, sequence length
- Total record count
- Original file size for validation

## Performance Notes

- Index building: Single pass through file
- Lookup: O(1) HashMap access + single seek/read
- Memory-mapped mode: No file I/O after initial mapping
- Index file size: ~100 bytes per record
- Supports files with millions of reads