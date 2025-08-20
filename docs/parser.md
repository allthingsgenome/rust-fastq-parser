# Parser Module

Core parsing logic and algorithms for FASTQ format.

## Overview

The parser module implements the low-level parsing engine that converts raw bytes into structured FASTQ records. It provides both zero-copy and allocating parsing modes, with optimizations for performance.

## Parser Struct

The main parsing engine that implements the FASTQ format state machine.

```rust
pub struct Parser {
    strict_mode: bool,
    allow_empty_lines: bool,
    validate_quality: bool,
}
```

### Constructor

```rust
impl Parser {
    pub fn new() -> Self {
        Self {
            strict_mode: true,
            allow_empty_lines: false,
            validate_quality: true,
        }
    }
    
    pub fn lenient() -> Self {
        Self {
            strict_mode: false,
            allow_empty_lines: true,
            validate_quality: false,
        }
    }
}
```

### Methods

#### parse

Parse a complete FASTQ file into records.

```rust
pub fn parse(&self, data: &[u8]) -> Result<Vec<FastqRecord>>
```

**Example:**
```rust
let parser = Parser::new();
let data = b"@seq1\nACGT\n+\nIIII\n";
let records = parser.parse(data)?;
```

#### parse_zero_copy

Parse without allocating new strings, returning references into the original data.

```rust
pub fn parse_zero_copy<'a>(&self, data: &'a [u8]) -> Result<Vec<FastqRecord<'a>>>
```

**Example:**
```rust
let parser = Parser::new();
let records = parser.parse_zero_copy(data)?;
// Records contain &[u8] slices into 'data'
```

#### parse_single

Parse a single FASTQ record.

```rust
pub fn parse_single(&self, data: &[u8]) -> Result<(FastqRecord, usize)>
```

Returns the parsed record and the number of bytes consumed.

**Example:**
```rust
let parser = Parser::new();
let (record, bytes_read) = parser.parse_single(data)?;
```

#### find_record_boundaries

Locate FASTQ record boundaries for parallel processing.

```rust
pub fn find_record_boundaries(&self, data: &[u8]) -> Vec<usize>
```

**Example:**
```rust
let boundaries = parser.find_record_boundaries(data);
// Use for splitting data into chunks
```

## Parsing Algorithm

The parser uses a state machine with four states:

```rust
enum ParseState {
    ExpectingHeader,    // '@' line
    ReadingSequence,    // Sequence line(s)
    ExpectingPlus,      // '+' line
    ReadingQuality,     // Quality line(s)
}
```

### State Transitions

1. **ExpectingHeader** → **ReadingSequence**: On '@' character
2. **ReadingSequence** → **ExpectingPlus**: On complete sequence
3. **ExpectingPlus** → **ReadingQuality**: On '+' character
4. **ReadingQuality** → **ExpectingHeader**: On complete quality

## Line Scanner

Efficient line scanning using memchr.

```rust
pub struct LineScanner<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> LineScanner<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }
    
    pub fn next_line(&mut self) -> Option<&'a [u8]> {
        // Uses memchr for fast newline detection
    }
}
```

## Format Validation

### Strict Mode

In strict mode, the parser enforces:

1. Header lines must start with '@'
2. Plus lines must start with '+'
3. Sequence and quality must have equal length
4. No empty lines between records
5. Valid ASCII characters in sequences
6. Valid quality score range

### Lenient Mode

In lenient mode, the parser:

1. Allows empty lines
2. Skips invalid records
3. Accepts wider quality score ranges
4. Tolerates malformed headers

## Optimizations

### Vectorized Operations

The parser uses SIMD when available:

```rust
fn find_newlines_simd(data: &[u8]) -> Vec<usize> {
    // AVX2 implementation for 32-byte chunks
}
```

### Memory Efficiency

1. **Zero-copy parsing**: Returns views into original data
2. **Lazy allocation**: Only allocates when necessary
3. **Buffer reuse**: Reuses internal buffers

### Performance Characteristics

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Parse single record | O(n) | n = record size |
| Find boundaries | O(n) | Single pass |
| Validate format | O(n) | Integrated with parsing |
| Zero-copy parse | O(n) | No allocations |

## Error Recovery

The parser provides error recovery mechanisms:

```rust
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub kind: ParseErrorKind,
}

pub enum ParseErrorKind {
    InvalidHeader,
    MismatchedLengths,
    InvalidCharacter(u8),
    UnexpectedEof,
}
```

### Recovery Strategies

```rust
impl Parser {
    pub fn parse_with_recovery(&self, data: &[u8]) -> (Vec<FastqRecord>, Vec<ParseError>) {
        // Returns valid records and errors
    }
    
    pub fn skip_invalid(&mut self, data: &[u8]) -> Result<Vec<FastqRecord>> {
        // Skips invalid records
    }
}
```

## Chunk Processing

For parallel processing support:

```rust
pub struct ChunkParser {
    parser: Parser,
    chunk_size: usize,
}

impl ChunkParser {
    pub fn split_chunks(&self, data: &[u8]) -> Vec<&[u8]> {
        // Splits at record boundaries
    }
    
    pub fn parse_chunk(&self, chunk: &[u8]) -> Result<Vec<FastqRecord>> {
        // Parses a single chunk
    }
}
```

## Configuration

Parser configuration options:

```rust
pub struct ParserConfig {
    pub strict_mode: bool,
    pub allow_empty_lines: bool,
    pub validate_quality: bool,
    pub max_record_size: Option<usize>,
    pub buffer_size: usize,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            allow_empty_lines: false,
            validate_quality: true,
            max_record_size: None,
            buffer_size: 8192,
        }
    }
}
```

## Examples

### Basic Parsing

```rust
use fastq_parser::Parser;

fn parse_fastq(data: &[u8]) -> Result<()> {
    let parser = Parser::new();
    let records = parser.parse(data)?;
    
    for record in records {
        println!("ID: {}", std::str::from_utf8(&record.id)?);
    }
    
    Ok(())
}
```

### Zero-Copy Parsing

```rust
use fastq_parser::Parser;

fn zero_copy_parse(data: &[u8]) -> Result<()> {
    let parser = Parser::new();
    let records = parser.parse_zero_copy(data)?;
    
    // No allocations - records reference original data
    for record in records {
        process_record_view(&record);
    }
    
    Ok(())
}
```

### Lenient Parsing

```rust
use fastq_parser::Parser;

fn parse_messy_file(data: &[u8]) -> Result<()> {
    let parser = Parser::lenient();
    let (records, errors) = parser.parse_with_recovery(data);
    
    println!("Parsed {} records with {} errors", 
             records.len(), errors.len());
    
    Ok(())
}
```

## See Also

- [Reader Module](./reader.md) - File I/O operations
- [Record Module](./record.md) - FASTQ record structure
- [SIMD Module](./simd.md) - Vectorized operations