# Error Module

Comprehensive error handling for FASTQ parsing.

## Overview

The error module provides a unified error type that covers all possible failure modes in the FASTQ parser. It includes detailed error information, error chaining, and recovery suggestions.

## Error Enum

The main error type for the library.

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid FASTQ format: {0}")]
    InvalidFormat(String),
    
    #[error("Invalid header line: expected '@', found '{0}'")]
    InvalidHeader(char),
    
    #[error("Invalid plus line: expected '+', found '{0}'")]
    InvalidPlusLine(char),
    
    #[error("Sequence and quality lengths don't match: seq={0}, qual={1}")]
    MismatchedLengths(usize, usize),
    
    #[error("Invalid quality score: {0}")]
    InvalidQualityScore(u8),
    
    #[error("Invalid nucleotide character: {0}")]
    InvalidNucleotide(u8),
    
    #[error("Incomplete record at line {0}")]
    IncompleteRecord(usize),
    
    #[error("Invalid UTF-8 in {context}")]
    InvalidUtf8 { context: String },
    
    #[error("Compression error: {0}")]
    CompressionError(String),
    
    #[error("Memory map error: {0}")]
    MmapError(String),
    
    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        line: usize,
        column: usize,
        message: String,
    },
    
    #[error("Parallel processing error: {0}")]
    ParallelError(String),
    
    #[error("Buffer overflow: tried to write {attempted} bytes to buffer of size {capacity}")]
    BufferOverflow {
        attempted: usize,
        capacity: usize,
    },
    
    #[error("Unexpected end of file")]
    UnexpectedEof,
    
    #[error("Feature not supported: {0}")]
    NotSupported(String),
}
```

## Result Type

Type alias for Results with our Error type.

```rust
pub type Result<T> = std::result::Result<T, Error>;
```

## Error Context

Additional context for errors.

```rust
pub struct ErrorContext {
    pub file_path: Option<PathBuf>,
    pub record_number: Option<usize>,
    pub byte_offset: Option<usize>,
    pub line_number: Option<usize>,
}

impl Error {
    pub fn with_context(self, context: ErrorContext) -> DetailedError {
        DetailedError {
            error: self,
            context,
        }
    }
}
```

## DetailedError

Error with additional context information.

```rust
pub struct DetailedError {
    error: Error,
    context: ErrorContext,
}

impl fmt::Display for DetailedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)?;
        
        if let Some(ref path) = self.context.file_path {
            write!(f, "\n  File: {}", path.display())?;
        }
        
        if let Some(record) = self.context.record_number {
            write!(f, "\n  Record: {}", record)?;
        }
        
        if let Some(line) = self.context.line_number {
            write!(f, "\n  Line: {}", line)?;
        }
        
        Ok(())
    }
}
```

## Error Recovery

Strategies for recovering from errors.

```rust
pub enum RecoveryStrategy {
    Skip,           // Skip the problematic record
    Abort,          // Stop processing immediately
    TryFix,         // Attempt to fix the issue
    Ignore,         // Continue as if nothing happened
    RetryWithLenient, // Retry with lenient parsing
}

impl Error {
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            Error::InvalidFormat(_) => RecoveryStrategy::Skip,
            Error::MismatchedLengths(_, _) => RecoveryStrategy::TryFix,
            Error::InvalidQualityScore(_) => RecoveryStrategy::RetryWithLenient,
            Error::Io(_) => RecoveryStrategy::Abort,
            Error::UnexpectedEof => RecoveryStrategy::Abort,
            _ => RecoveryStrategy::Skip,
        }
    }
    
    pub fn is_recoverable(&self) -> bool {
        !matches!(self.recovery_strategy(), RecoveryStrategy::Abort)
    }
}
```

## Error Collection

Collect multiple errors during parsing.

```rust
pub struct ErrorCollector {
    errors: Vec<DetailedError>,
    max_errors: Option<usize>,
}

impl ErrorCollector {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            max_errors: None,
        }
    }
    
    pub fn with_max_errors(max: usize) -> Self {
        Self {
            errors: Vec::new(),
            max_errors: Some(max),
        }
    }
    
    pub fn add(&mut self, error: DetailedError) -> bool {
        self.errors.push(error);
        
        if let Some(max) = self.max_errors {
            self.errors.len() >= max
        } else {
            false
        }
    }
    
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
    
    pub fn take_errors(self) -> Vec<DetailedError> {
        self.errors
    }
}
```

## Validation Errors

Specific errors for format validation.

```rust
#[derive(Debug)]
pub enum ValidationError {
    EmptyIdentifier,
    InvalidIdentifierFormat(String),
    EmptySequence,
    InvalidSequenceCharacter(char, usize),
    EmptyQuality,
    QualityLengthMismatch { seq_len: usize, qual_len: usize },
    InvalidQualityCharacter(char, usize),
    QualityOutOfRange(u8),
}

impl From<ValidationError> for Error {
    fn from(err: ValidationError) -> Self {
        match err {
            ValidationError::EmptyIdentifier => 
                Error::InvalidFormat("Empty identifier".to_string()),
            ValidationError::InvalidIdentifierFormat(msg) => 
                Error::InvalidFormat(format!("Invalid identifier: {}", msg)),
            // ... other conversions
        }
    }
}
```

## Error Chain

Chain errors with causes.

```rust
pub struct ErrorChain {
    errors: Vec<Error>,
}

impl ErrorChain {
    pub fn new(error: Error) -> Self {
        Self {
            errors: vec![error],
        }
    }
    
    pub fn caused_by(mut self, error: Error) -> Self {
        self.errors.push(error);
        self
    }
    
    pub fn root_cause(&self) -> &Error {
        self.errors.last().unwrap()
    }
}

impl fmt::Display for ErrorChain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, error) in self.errors.iter().enumerate() {
            if i == 0 {
                write!(f, "Error: {}", error)?;
            } else {
                write!(f, "\n  Caused by: {}", error)?;
            }
        }
        Ok(())
    }
}
```

## Error Reporting

User-friendly error reporting.

```rust
pub struct ErrorReport {
    error: DetailedError,
    suggestion: Option<String>,
}

impl ErrorReport {
    pub fn new(error: DetailedError) -> Self {
        let suggestion = Self::suggest_fix(&error.error);
        Self { error, suggestion }
    }
    
    fn suggest_fix(error: &Error) -> Option<String> {
        match error {
            Error::InvalidFormat(_) => 
                Some("Check that the file is a valid FASTQ format".to_string()),
            Error::MismatchedLengths(_, _) => 
                Some("Ensure sequence and quality strings have the same length".to_string()),
            Error::InvalidQualityScore(_) => 
                Some("Check the quality encoding (Phred+33 or Phred+64)".to_string()),
            Error::CompressionError(_) => 
                Some("Try decompressing the file manually first".to_string()),
            _ => None,
        }
    }
    
    pub fn print_report(&self) {
        eprintln!("{}", self.error);
        
        if let Some(ref suggestion) = self.suggestion {
            eprintln!("\nSuggestion: {}", suggestion);
        }
    }
}
```

## Custom Error Types

Define domain-specific errors.

```rust
#[derive(Debug, thiserror::Error)]
pub enum QualityError {
    #[error("Quality score {score} out of range [{min}, {max}]")]
    OutOfRange { score: u8, min: u8, max: u8 },
    
    #[error("Invalid quality encoding")]
    InvalidEncoding,
    
    #[error("Quality string too short: expected {expected}, got {actual}")]
    TooShort { expected: usize, actual: usize },
}

impl From<QualityError> for Error {
    fn from(err: QualityError) -> Self {
        Error::InvalidFormat(err.to_string())
    }
}
```

## Examples

### Basic Error Handling

```rust
use fastq_parser::{FastqReader, Error, Result};

fn parse_file(path: &str) -> Result<()> {
    let reader = FastqReader::from_path(path)?;
    
    for (i, result) in reader.into_records().enumerate() {
        match result {
            Ok(record) => process_record(record),
            Err(Error::InvalidFormat(msg)) => {
                eprintln!("Skipping invalid record {}: {}", i + 1, msg);
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    
    Ok(())
}
```

### Error Collection

```rust
use fastq_parser::{FastqReader, ErrorCollector};

fn parse_with_errors(path: &str) -> Result<(Vec<FastqRecord>, Vec<DetailedError>)> {
    let reader = FastqReader::from_path(path)?;
    let mut collector = ErrorCollector::with_max_errors(100);
    let mut records = Vec::new();
    
    for (i, result) in reader.into_records().enumerate() {
        match result {
            Ok(record) => records.push(record),
            Err(e) => {
                let detailed = e.with_context(ErrorContext {
                    file_path: Some(path.into()),
                    record_number: Some(i + 1),
                    ..Default::default()
                });
                
                if collector.add(detailed) {
                    break; // Max errors reached
                }
            }
        }
    }
    
    Ok((records, collector.take_errors()))
}
```

### Recovery Strategy

```rust
use fastq_parser::{Parser, Error, RecoveryStrategy};

fn parse_with_recovery(data: &[u8]) -> Vec<FastqRecord> {
    let mut parser = Parser::new();
    let mut records = Vec::new();
    let mut offset = 0;
    
    while offset < data.len() {
        match parser.parse_single(&data[offset..]) {
            Ok((record, consumed)) => {
                records.push(record);
                offset += consumed;
            }
            Err(e) => {
                match e.recovery_strategy() {
                    RecoveryStrategy::Skip => {
                        // Skip to next record
                        offset = find_next_record(&data[offset..])
                            .map(|o| offset + o)
                            .unwrap_or(data.len());
                    }
                    RecoveryStrategy::RetryWithLenient => {
                        parser = Parser::lenient();
                        // Retry with lenient parser
                    }
                    RecoveryStrategy::Abort => break,
                    _ => offset += 1, // Skip one byte and try again
                }
            }
        }
    }
    
    records
}
```

### Detailed Error Reporting

```rust
use fastq_parser::{FastqReader, ErrorReport};

fn main() {
    if let Err(e) = process_fastq("input.fastq") {
        let detailed = e.with_context(ErrorContext {
            file_path: Some("input.fastq".into()),
            ..Default::default()
        });
        
        let report = ErrorReport::new(detailed);
        report.print_report();
        
        std::process::exit(1);
    }
}
```

## See Also

- [Parser Module](./parser.md) - Error generation during parsing
- [Reader Module](./reader.md) - I/O error handling
- [Record Module](./record.md) - Validation errors