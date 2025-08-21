pub mod barcode;
pub mod buffer;
pub mod error;
pub mod filter;
pub mod index;
pub mod metrics;
pub mod paired;
pub mod parallel;
pub mod parser;
pub mod reader;
pub mod record;
pub mod simd;
pub mod stream;
pub mod writer;

pub use barcode::{
    BarcodeConfig, BarcodeCorrector, BarcodeExtractor, Demultiplexer, UmiDeduplicator,
};
pub use error::{FastqError, Result};
pub use filter::{AdapterTrimmer, AdvancedFilter, FilterStats, QualityFilter};
pub use index::{FastqIndex, IndexedReader, RandomAccessReader};
pub use metrics::{ErrorDetector, QualityMetrics, QualityPlotter};
pub use paired::{InterleavedReader, PairedEndReader};
pub use parser::{Parser, ParserBuilder};
pub use reader::{FastqReader, FastqReaderBuilder};
pub use record::{OwnedRecord, QualityEncoding, Record};
pub use stream::{AsyncStreamingReader, ChunkedStreamer, StreamingReader};
pub use writer::{FastaWriter, FastqWriter, FormatConverter, SubsetExtractor};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let data = b"@SEQ_ID\nGATTTGGGGTTCAAAGCAGTATCGATCAAATAGTAAATCCATTTGTTCAACTCACAGTTT\n+\n!''*((((***+))%%%++)(%%%%).1***-+*''))**55CCF>>>>>>CCCCCCC65\n";
        let parser = Parser::new(data);
        let records: Vec<_> = parser.collect();
        assert_eq!(records.len(), 1);
    }
}
