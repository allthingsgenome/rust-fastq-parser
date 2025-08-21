use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityEncoding {
    Phred33,
    Phred64,
    Unknown,
}

impl QualityEncoding {
    pub fn detect(qual_string: &[u8]) -> Self {
        let min_qual = qual_string.iter().min().copied().unwrap_or(b'!');
        let max_qual = qual_string.iter().max().copied().unwrap_or(b'~');

        if min_qual < b'!' || max_qual > b'~' {
            return QualityEncoding::Unknown;
        }

        if min_qual < b';' {
            QualityEncoding::Phred33
        } else if min_qual >= b'@' && max_qual > b'h' {
            QualityEncoding::Phred64
        } else {
            QualityEncoding::Phred33
        }
    }

    pub fn to_phred_scores(&self, qual_string: &[u8]) -> Vec<u8> {
        match self {
            QualityEncoding::Phred33 => qual_string.iter().map(|&q| q.saturating_sub(33)).collect(),
            QualityEncoding::Phred64 => qual_string.iter().map(|&q| q.saturating_sub(64)).collect(),
            QualityEncoding::Unknown => {
                vec![0; qual_string.len()]
            }
        }
    }

    pub fn offset(&self) -> u8 {
        match self {
            QualityEncoding::Phred33 => 33,
            QualityEncoding::Phred64 => 64,
            QualityEncoding::Unknown => 33,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record<'a> {
    pub id: &'a [u8],
    pub desc: Option<&'a [u8]>,
    pub seq: &'a [u8],
    pub qual: &'a [u8],
    quality_encoding: Option<QualityEncoding>,
}

impl<'a> Record<'a> {
    #[inline]
    pub fn new(id: &'a [u8], desc: Option<&'a [u8]>, seq: &'a [u8], qual: &'a [u8]) -> Self {
        Record {
            id,
            desc,
            seq,
            qual,
            quality_encoding: None,
        }
    }

    #[inline]
    pub fn id(&self) -> &[u8] {
        self.id
    }

    #[inline]
    pub fn id_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.id)
    }

    #[inline]
    pub fn desc(&self) -> Option<&[u8]> {
        self.desc
    }

    #[inline]
    pub fn desc_str(&self) -> Option<Result<&str, std::str::Utf8Error>> {
        self.desc.map(std::str::from_utf8)
    }

    #[inline]
    pub fn seq(&self) -> &[u8] {
        self.seq
    }

    #[inline]
    pub fn seq_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.seq)
    }

    #[inline]
    pub fn qual(&self) -> &[u8] {
        self.qual
    }

    #[inline]
    pub fn qual_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.qual)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.seq.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.seq.is_empty()
    }

    #[inline]
    pub fn validate(&self) -> Result<(), crate::error::FastqError> {
        if self.seq.len() != self.qual.len() {
            return Err(crate::error::FastqError::LengthMismatch {
                seq_len: self.seq.len(),
                qual_len: self.qual.len(),
            });
        }

        for &base in self.seq {
            if !matches!(
                base,
                b'A' | b'C' | b'G' | b'T' | b'N' | b'a' | b'c' | b'g' | b't' | b'n'
            ) {
                return Err(crate::error::FastqError::InvalidBase { base });
            }
        }

        for &qual in self.qual {
            if !(b'!'..=b'~').contains(&qual) {
                return Err(crate::error::FastqError::InvalidQuality { qual });
            }
        }

        Ok(())
    }

    pub fn quality_encoding(&mut self) -> QualityEncoding {
        if self.quality_encoding.is_none() {
            self.quality_encoding = Some(QualityEncoding::detect(self.qual));
        }
        self.quality_encoding.unwrap()
    }

    pub fn phred_scores(&mut self) -> Vec<u8> {
        let encoding = self.quality_encoding();
        encoding.to_phred_scores(self.qual)
    }

    pub fn mean_quality(&mut self) -> f64 {
        let scores = self.phred_scores();
        if scores.is_empty() {
            return 0.0;
        }
        scores.iter().map(|&q| q as f64).sum::<f64>() / scores.len() as f64
    }
}

impl<'a> fmt::Display for Record<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@")?;
        f.write_str(self.id_str().map_err(|_| fmt::Error)?)?;
        if let Some(desc) = self.desc_str() {
            write!(f, " {}", desc.map_err(|_| fmt::Error)?)?;
        }
        writeln!(f)?;
        f.write_str(self.seq_str().map_err(|_| fmt::Error)?)?;
        write!(f, "\n+\n")?;
        f.write_str(self.qual_str().map_err(|_| fmt::Error)?)?;
        Ok(())
    }
}

pub struct OwnedRecord {
    pub id: Vec<u8>,
    pub desc: Option<Vec<u8>>,
    pub seq: Vec<u8>,
    pub qual: Vec<u8>,
}

impl OwnedRecord {
    pub fn from_record(record: &Record) -> Self {
        OwnedRecord {
            id: record.id.to_vec(),
            desc: record.desc.map(|d| d.to_vec()),
            seq: record.seq.to_vec(),
            qual: record.qual.to_vec(),
        }
    }

    pub fn as_record(&self) -> Record<'_> {
        Record {
            id: &self.id,
            desc: self.desc.as_deref(),
            seq: &self.seq,
            qual: &self.qual,
            quality_encoding: None,
        }
    }
}
