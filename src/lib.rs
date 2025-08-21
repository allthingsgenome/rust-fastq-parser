pub mod error;
pub mod record;
pub mod parser;
pub mod reader;
pub mod buffer;
pub mod simd;
pub mod parallel;
pub mod filter;
pub mod stream;
pub mod paired;
pub mod writer;
pub mod index;
pub mod barcode;
pub mod metrics;

pub use error::{FastqError, Result};
pub use record::{Record, QualityEncoding, OwnedRecord};
pub use parser::{Parser, ParserBuilder};
pub use reader::{FastqReader, FastqReaderBuilder};
pub use filter::{QualityFilter, AdapterTrimmer, FilterStats, AdvancedFilter};
pub use stream::{StreamingReader, AsyncStreamingReader, ChunkedStreamer};
pub use paired::{PairedEndReader, InterleavedReader};
pub use writer::{FastqWriter, FastaWriter, FormatConverter, SubsetExtractor};
pub use index::{FastqIndex, IndexedReader, RandomAccessReader};
pub use barcode::{BarcodeConfig, BarcodeExtractor, Demultiplexer, UmiDeduplicator, BarcodeCorrector};
pub use metrics::{QualityMetrics, ErrorDetector, QualityPlotter};

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