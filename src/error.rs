use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FastqError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid FASTQ format at line {line}: {msg}")]
    InvalidFormat { line: usize, msg: String },

    #[error("Sequence and quality lengths don't match (seq: {seq_len}, qual: {qual_len})")]
    LengthMismatch { seq_len: usize, qual_len: usize },

    #[error("Invalid header: expected '@' at line {line}")]
    InvalidHeader { line: usize },

    #[error("Invalid separator: expected '+' at line {line}")]
    InvalidSeparator { line: usize },

    #[error("Unexpected end of file")]
    UnexpectedEof,

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Invalid base character: {base}")]
    InvalidBase { base: u8 },

    #[error("Invalid quality character: {qual}")]
    InvalidQuality { qual: u8 },

    #[error("Paired-end read ID mismatch: R1={r1_id}, R2={r2_id}")]
    PairedEndMismatch { r1_id: String, r2_id: String },

    #[error("Paired-end files have different number of reads")]
    PairedEndLengthMismatch,

    #[error("Interleaved file has odd number of reads")]
    InterleavedOddCount,
}

pub type Result<T> = std::result::Result<T, FastqError>;
