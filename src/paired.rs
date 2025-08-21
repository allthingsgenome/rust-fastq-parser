use crate::{
    error::{FastqError, Result},
    reader::FastqReader,
    record::{OwnedRecord, Record},
};
use std::path::Path;

pub struct PairedEndReader {
    r1_reader: FastqReader,
    r2_reader: FastqReader,
}

impl PairedEndReader {
    pub fn from_paths<P: AsRef<Path>>(r1_path: P, r2_path: P) -> Result<Self> {
        let r1_reader = FastqReader::from_path(r1_path)?;
        let r2_reader = FastqReader::from_path(r2_path)?;

        Ok(PairedEndReader {
            r1_reader,
            r2_reader,
        })
    }

    pub fn into_paired_records(self) -> PairedRecordIterator {
        PairedRecordIterator {
            r1_iter: self.r1_reader.into_records(),
            r2_iter: self.r2_reader.into_records(),
            strict_pairing: true,
        }
    }

    pub fn validate_pairing(self) -> Result<bool> {
        let mut r1_iter = self.r1_reader.into_records();
        let mut r2_iter = self.r2_reader.into_records();

        let mut count = 0;
        const SAMPLE_SIZE: usize = 1000;

        while count < SAMPLE_SIZE {
            match (r1_iter.next(), r2_iter.next()) {
                (Some(Ok(r1)), Some(Ok(r2))) => {
                    if !Self::ids_match(&r1.as_record(), &r2.as_record()) {
                        return Ok(false);
                    }
                    count += 1;
                }
                (None, None) => break,
                (Some(_), None) | (None, Some(_)) => return Ok(false),
                (Some(Err(e)), _) | (_, Some(Err(e))) => return Err(e),
            }
        }

        Ok(true)
    }

    fn ids_match(r1: &Record, r2: &Record) -> bool {
        let id1 = Self::extract_base_id(r1.id());
        let id2 = Self::extract_base_id(r2.id());
        id1 == id2
    }

    fn extract_base_id(id: &[u8]) -> &[u8] {
        if let Some(space_pos) = id.iter().position(|&b| b == b' ') {
            &id[..space_pos]
        } else if let Some(slash_pos) = id.iter().position(|&b| b == b'/') {
            &id[..slash_pos]
        } else {
            id
        }
    }
}

pub struct PairedRecordIterator {
    r1_iter: Box<dyn Iterator<Item = Result<OwnedRecord>> + Send>,
    r2_iter: Box<dyn Iterator<Item = Result<OwnedRecord>> + Send>,
    strict_pairing: bool,
}

impl PairedRecordIterator {
    pub fn strict_pairing(mut self, strict: bool) -> Self {
        self.strict_pairing = strict;
        self
    }
}

impl Iterator for PairedRecordIterator {
    type Item = Result<(OwnedRecord, OwnedRecord)>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.r1_iter.next(), self.r2_iter.next()) {
            (Some(Ok(r1)), Some(Ok(r2))) => {
                if self.strict_pairing {
                    let id1 = PairedEndReader::extract_base_id(&r1.id);
                    let id2 = PairedEndReader::extract_base_id(&r2.id);

                    if id1 != id2 {
                        return Some(Err(FastqError::PairedEndMismatch {
                            r1_id: String::from_utf8_lossy(&r1.id).into_owned(),
                            r2_id: String::from_utf8_lossy(&r2.id).into_owned(),
                        }));
                    }
                }
                Some(Ok((r1, r2)))
            }
            (Some(Err(e)), _) | (_, Some(Err(e))) => Some(Err(e)),
            (None, None) => None,
            (Some(_), None) | (None, Some(_)) => Some(Err(FastqError::PairedEndLengthMismatch)),
        }
    }
}

pub struct InterleavedReader {
    reader: FastqReader,
}

impl InterleavedReader {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let reader = FastqReader::from_path(path)?;
        Ok(InterleavedReader { reader })
    }

    pub fn into_paired_records(self) -> InterleavedPairedIterator {
        InterleavedPairedIterator {
            iter: self.reader.into_records(),
        }
    }
}

pub struct InterleavedPairedIterator {
    iter: Box<dyn Iterator<Item = Result<OwnedRecord>> + Send>,
}

impl Iterator for InterleavedPairedIterator {
    type Item = Result<(OwnedRecord, OwnedRecord)>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.iter.next(), self.iter.next()) {
            (Some(Ok(r1)), Some(Ok(r2))) => Some(Ok((r1, r2))),
            (Some(Err(e)), _) | (_, Some(Err(e))) => Some(Err(e)),
            (None, None) | (Some(_), None) => None,
            (None, Some(_)) => Some(Err(FastqError::InterleavedOddCount)),
        }
    }
}
