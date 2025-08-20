# Memory-Mapped I/O Module

Efficient file handling using memory-mapped I/O for large FASTQ files.

## Overview

The mmap module provides memory-mapped file access, allowing the parser to work with large files without loading them entirely into RAM. This approach provides fast random access and reduces memory pressure.

## MmapReader Struct

Memory-mapped file reader.

```rust
pub struct MmapReader {
    mmap: Mmap,
    file: File,
    len: usize,
}
```

### Constructor

```rust
impl MmapReader {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let len = metadata.len() as usize;
        
        let mmap = unsafe {
            MmapOptions::new()
                .len(len)
                .map(&file)?
        };
        
        Ok(Self { mmap, file, len })
    }
    
    pub fn with_options<P: AsRef<Path>>(path: P, options: MmapOptions) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { options.map(&file)? };
        let len = mmap.len();
        
        Ok(Self { mmap, file, len })
    }
}
```

### Methods

#### as_slice

Get the memory-mapped data as a byte slice.

```rust
pub fn as_slice(&self) -> &[u8] {
    &self.mmap[..]
}
```

#### len

Get the file size.

```rust
pub fn len(&self) -> usize {
    self.len
}
```

#### advise

Provide hints to the kernel about access patterns.

```rust
pub fn advise(&self, advice: Advice) -> Result<()> {
    self.mmap.advise(advice)?;
    Ok(())
}
```

## MmapOptions

Configuration for memory mapping.

```rust
pub struct MmapOptions {
    populate: bool,
    lock: bool,
    sequential: bool,
    prefetch: bool,
    huge_pages: bool,
}

impl Default for MmapOptions {
    fn default() -> Self {
        Self {
            populate: false,
            lock: false,
            sequential: true,
            prefetch: true,
            huge_pages: false,
        }
    }
}

impl MmapOptions {
    pub fn populate(mut self, populate: bool) -> Self {
        self.populate = populate;
        self
    }
    
    pub fn lock(mut self, lock: bool) -> Self {
        self.lock = lock;
        self
    }
    
    pub fn sequential(mut self, sequential: bool) -> Self {
        self.sequential = sequential;
        self
    }
    
    pub fn prefetch(mut self, prefetch: bool) -> Self {
        self.prefetch = prefetch;
        self
    }
    
    pub fn huge_pages(mut self, huge_pages: bool) -> Self {
        self.huge_pages = huge_pages;
        self
    }
}
```

## Advice Enum

Memory access pattern hints.

```rust
#[derive(Debug, Clone, Copy)]
pub enum Advice {
    Normal,      // No special treatment
    Sequential,  // Sequential access expected
    Random,      // Random access expected
    WillNeed,    // Data will be needed soon
    DontNeed,    // Data won't be needed soon
}

impl Advice {
    fn to_libc(&self) -> libc::c_int {
        match self {
            Advice::Normal => libc::MADV_NORMAL,
            Advice::Sequential => libc::MADV_SEQUENTIAL,
            Advice::Random => libc::MADV_RANDOM,
            Advice::WillNeed => libc::MADV_WILLNEED,
            Advice::DontNeed => libc::MADV_DONTNEED,
        }
    }
}
```

## MmapParser

Parser specifically for memory-mapped files.

```rust
pub struct MmapParser {
    reader: MmapReader,
    parser: Parser,
}

impl MmapParser {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let reader = MmapReader::new(path)?;
        let parser = Parser::new();
        
        Ok(Self { reader, parser })
    }
    
    pub fn parse(&self) -> Result<Vec<FastqRecord>> {
        // Advise sequential access
        self.reader.advise(Advice::Sequential)?;
        
        // Parse the memory-mapped data
        self.parser.parse(self.reader.as_slice())
    }
    
    pub fn parse_range(&self, start: usize, end: usize) -> Result<Vec<FastqRecord>> {
        let data = &self.reader.as_slice()[start..end];
        self.parser.parse(data)
    }
}
```

## Windowed Access

Access memory-mapped file in windows.

```rust
pub struct MmapWindow {
    mmap: MmapReader,
    window_size: usize,
    position: usize,
}

impl MmapWindow {
    pub fn new(mmap: MmapReader, window_size: usize) -> Self {
        Self {
            mmap,
            window_size,
            position: 0,
        }
    }
    
    pub fn next_window(&mut self) -> Option<&[u8]> {
        if self.position >= self.mmap.len() {
            return None;
        }
        
        let end = (self.position + self.window_size).min(self.mmap.len());
        let window = &self.mmap.as_slice()[self.position..end];
        
        // Find actual end at record boundary
        let actual_end = self.find_record_boundary(window);
        self.position += actual_end;
        
        Some(&window[..actual_end])
    }
    
    fn find_record_boundary(&self, window: &[u8]) -> usize {
        // Look for '@' at start of line, indicating new record
        for i in (0..window.len()).rev() {
            if i == 0 || (window[i] == b'@' && window[i-1] == b'\n') {
                return i;
            }
        }
        window.len()
    }
}
```

## Prefetching

Prefetch data for better performance.

```rust
pub struct PrefetchingMmap {
    mmap: MmapReader,
    prefetch_distance: usize,
    current_offset: AtomicUsize,
}

impl PrefetchingMmap {
    pub fn new(mmap: MmapReader) -> Self {
        Self {
            mmap,
            prefetch_distance: 1024 * 1024, // 1MB
            current_offset: AtomicUsize::new(0),
        }
    }
    
    pub fn read_at(&self, offset: usize, len: usize) -> &[u8] {
        // Prefetch next chunk
        let next_offset = offset + len;
        if next_offset < self.mmap.len() {
            self.prefetch(next_offset, self.prefetch_distance);
        }
        
        // Update current offset
        self.current_offset.store(offset + len, Ordering::Relaxed);
        
        &self.mmap.as_slice()[offset..offset + len]
    }
    
    fn prefetch(&self, offset: usize, len: usize) {
        #[cfg(target_os = "linux")]
        unsafe {
            let ptr = self.mmap.as_slice().as_ptr().add(offset);
            let len = len.min(self.mmap.len() - offset);
            libc::madvise(ptr as *mut _, len, libc::MADV_WILLNEED);
        }
    }
}
```

## Page-Aligned Access

Optimize for page-aligned access.

```rust
pub struct PageAlignedMmap {
    mmap: MmapReader,
    page_size: usize,
}

impl PageAlignedMmap {
    pub fn new(mmap: MmapReader) -> Self {
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
        
        Self { mmap, page_size }
    }
    
    pub fn aligned_window(&self, offset: usize, len: usize) -> (usize, usize) {
        // Align start to page boundary
        let aligned_start = (offset / self.page_size) * self.page_size;
        
        // Align length to page boundary
        let end = offset + len;
        let aligned_end = ((end + self.page_size - 1) / self.page_size) * self.page_size;
        let aligned_len = aligned_end - aligned_start;
        
        (aligned_start, aligned_len)
    }
    
    pub fn read_aligned(&self, offset: usize, len: usize) -> &[u8] {
        let (aligned_offset, aligned_len) = self.aligned_window(offset, len);
        
        // Advise kernel about the access
        let data = &self.mmap.as_slice()[aligned_offset..aligned_offset + aligned_len];
        
        // Return the actually requested slice
        let start_offset = offset - aligned_offset;
        &data[start_offset..start_offset + len]
    }
}
```

## Huge Pages Support

Use huge pages for better TLB performance.

```rust
#[cfg(target_os = "linux")]
pub struct HugePageMmap {
    ptr: *mut u8,
    len: usize,
}

#[cfg(target_os = "linux")]
impl HugePageMmap {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len() as usize;
        
        // Allocate huge pages
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_PRIVATE | libc::MAP_HUGETLB,
                file.as_raw_fd(),
                0,
            ) as *mut u8
        };
        
        if ptr == libc::MAP_FAILED as *mut u8 {
            return Err(Error::MmapError("Failed to map huge pages".to_string()));
        }
        
        Ok(Self { ptr, len })
    }
    
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

#[cfg(target_os = "linux")]
impl Drop for HugePageMmap {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut _, self.len);
        }
    }
}
```

## Performance Monitoring

Track memory-mapped I/O statistics.

```rust
pub struct MmapStats {
    page_faults: AtomicUsize,
    bytes_read: AtomicUsize,
    access_count: AtomicUsize,
}

impl MmapStats {
    pub fn new() -> Self {
        Self {
            page_faults: AtomicUsize::new(0),
            bytes_read: AtomicUsize::new(0),
            access_count: AtomicUsize::new(0),
        }
    }
    
    pub fn record_access(&self, bytes: usize) {
        self.bytes_read.fetch_add(bytes, Ordering::Relaxed);
        self.access_count.fetch_add(1, Ordering::Relaxed);
    }
    
    #[cfg(target_os = "linux")]
    pub fn get_page_faults(&self) -> usize {
        // Read from /proc/self/stat
        // Implementation details...
        0
    }
}
```

## Examples

### Basic Memory Mapping

```rust
use fastq_parser::mmap::MmapReader;

fn parse_with_mmap(path: &str) -> Result<()> {
    let reader = MmapReader::new(path)?;
    let data = reader.as_slice();
    
    println!("File size: {} bytes", data.len());
    
    // Parse directly from memory-mapped data
    let parser = Parser::new();
    let records = parser.parse(data)?;
    
    Ok(())
}
```

### Sequential Access with Advice

```rust
use fastq_parser::mmap::{MmapReader, Advice};

fn sequential_parse(path: &str) -> Result<()> {
    let reader = MmapReader::new(path)?;
    
    // Advise sequential access
    reader.advise(Advice::Sequential)?;
    
    // Parse sequentially
    let mut offset = 0;
    while offset < reader.len() {
        let chunk = &reader.as_slice()[offset..];
        // Process chunk
        offset += process_chunk(chunk)?;
    }
    
    Ok(())
}
```

### Windowed Processing

```rust
use fastq_parser::mmap::{MmapReader, MmapWindow};

fn process_in_windows(path: &str) -> Result<()> {
    let reader = MmapReader::new(path)?;
    let mut window = MmapWindow::new(reader, 10 * 1024 * 1024); // 10MB windows
    
    while let Some(data) = window.next_window() {
        // Process window
        let records = parse_window(data)?;
        process_records(records);
    }
    
    Ok(())
}
```

### Prefetching for Performance

```rust
use fastq_parser::mmap::PrefetchingMmap;

fn parse_with_prefetch(path: &str) -> Result<()> {
    let reader = MmapReader::new(path)?;
    let prefetcher = PrefetchingMmap::new(reader);
    
    let chunk_size = 1024 * 1024; // 1MB chunks
    let mut offset = 0;
    
    while offset < prefetcher.mmap.len() {
        let len = chunk_size.min(prefetcher.mmap.len() - offset);
        let data = prefetcher.read_at(offset, len);
        
        // Process data (next chunk is being prefetched)
        process_data(data)?;
        
        offset += len;
    }
    
    Ok(())
}
```

## Performance Tips

1. **Use Sequential Advice**: For linear parsing, always use `Advice::Sequential`
2. **Page-Aligned Access**: Align reads to page boundaries when possible
3. **Prefetch Ahead**: Prefetch the next chunk while processing current
4. **Huge Pages**: Use huge pages for very large files (>100MB)
5. **Lock Pages**: Lock frequently accessed pages in memory
6. **Window Size**: Choose window size based on L3 cache size

## Platform Support

| Platform | Basic mmap | Advice | Huge Pages | Prefetch |
|----------|------------|--------|------------|----------|
| Linux | ✓ | ✓ | ✓ | ✓ |
| macOS | ✓ | ✓ | ✗ | ✓ |
| Windows | ✓ | Limited | ✗ | Limited |

## See Also

- [Reader Module](./reader.md) - Uses mmap for large files
- [Buffer Module](./buffer.md) - Alternative to mmap for small files
- [Parallel Module](./parallel.md) - Parallel processing of mmap data