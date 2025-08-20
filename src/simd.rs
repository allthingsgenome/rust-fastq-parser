#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::sync::OnceLock;

static HAS_AVX2: OnceLock<bool> = OnceLock::new();

#[inline]
fn has_avx2() -> bool {
    *HAS_AVX2.get_or_init(|| {
        #[cfg(target_arch = "x86_64")]
        {
            is_x86_feature_detected!("avx2")
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            false
        }
    })
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
pub unsafe fn find_newlines_avx2(data: &[u8]) -> Vec<usize> {
    let mut positions = Vec::new();
    let newline = _mm256_set1_epi8(b'\n' as i8);
    
    let chunks = data.chunks_exact(32);
    let remainder = chunks.remainder();
    
    for (chunk_idx, chunk) in chunks.enumerate() {
        let chunk_ptr = chunk.as_ptr() as *const __m256i;
        let vector = _mm256_loadu_si256(chunk_ptr);
        let cmp = _mm256_cmpeq_epi8(vector, newline);
        let mask = _mm256_movemask_epi8(cmp);
        
        if mask != 0 {
            let base = chunk_idx * 32;
            for i in 0..32 {
                if (mask & (1 << i)) != 0 {
                    positions.push(base + i);
                }
            }
        }
    }
    
    let offset = data.len() - remainder.len();
    for (i, &byte) in remainder.iter().enumerate() {
        if byte == b'\n' {
            positions.push(offset + i);
        }
    }
    
    positions
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
pub unsafe fn validate_ascii_avx2(data: &[u8]) -> bool {
    let max_ascii = _mm256_set1_epi8(127);
    
    let chunks = data.chunks_exact(32);
    let remainder = chunks.remainder();
    
    for chunk in chunks {
        let chunk_ptr = chunk.as_ptr() as *const __m256i;
        let vector = _mm256_loadu_si256(chunk_ptr);
        let cmp = _mm256_cmpgt_epi8(vector, max_ascii);
        if _mm256_movemask_epi8(cmp) != 0 {
            return false;
        }
    }
    
    remainder.iter().all(|&b| b <= 127)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
pub unsafe fn count_chars_avx2(data: &[u8], target: u8) -> usize {
    let mut count = 0;
    let target_vec = _mm256_set1_epi8(target as i8);
    
    let chunks = data.chunks_exact(32);
    let remainder = chunks.remainder();
    
    for chunk in chunks {
        let chunk_ptr = chunk.as_ptr() as *const __m256i;
        let vector = _mm256_loadu_si256(chunk_ptr);
        let cmp = _mm256_cmpeq_epi8(vector, target_vec);
        let mask = _mm256_movemask_epi8(cmp);
        count += mask.count_ones() as usize;
    }
    
    count + remainder.iter().filter(|&&b| b == target).count()
}

#[inline]
pub fn find_newlines(data: &[u8]) -> Vec<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        if has_avx2() {
            return unsafe { find_newlines_avx2(data) };
        }
    }
    
    data.iter()
        .enumerate()
        .filter_map(|(i, &b)| if b == b'\n' { Some(i) } else { None })
        .collect()
}

#[inline]
pub fn validate_ascii(data: &[u8]) -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        if has_avx2() {
            return unsafe { validate_ascii_avx2(data) };
        }
    }
    
    data.iter().all(|&b| b <= 127)
}

#[inline]
pub fn count_chars(data: &[u8], target: u8) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        if has_avx2() {
            return unsafe { count_chars_avx2(data, target) };
        }
    }
    
    bytecount::count(data, target)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
pub unsafe fn find_char_avx2(data: &[u8], target: u8, start: usize) -> Option<usize> {
    let target_vec = _mm256_set1_epi8(target as i8);
    let slice = &data[start..];
    
    let chunks = slice.chunks_exact(32);
    let remainder = chunks.remainder();
    
    for (chunk_idx, chunk) in chunks.enumerate() {
        let chunk_ptr = chunk.as_ptr() as *const __m256i;
        let vector = _mm256_loadu_si256(chunk_ptr);
        let cmp = _mm256_cmpeq_epi8(vector, target_vec);
        let mask = _mm256_movemask_epi8(cmp);
        
        if mask != 0 {
            let offset = mask.trailing_zeros() as usize;
            return Some(start + chunk_idx * 32 + offset);
        }
    }
    
    let remainder_start = slice.len() - remainder.len();
    remainder.iter()
        .position(|&b| b == target)
        .map(|i| start + remainder_start + i)
}

#[inline]
pub fn find_char(data: &[u8], target: u8, start: usize) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        if has_avx2() {
            return unsafe { find_char_avx2(data, target, start) };
        }
    }
    
    memchr::memchr(target, &data[start..]).map(|i| start + i)
}

pub mod bytecount {
    pub fn count(data: &[u8], byte: u8) -> usize {
        memchr::memchr_iter(byte, data).count()
    }
}