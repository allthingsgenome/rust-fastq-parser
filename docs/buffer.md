# Buffer Module

Efficient memory management and buffer pooling for FASTQ parsing.

## Overview

The buffer module provides memory management utilities to minimize allocations during parsing. It includes buffer pools for reusing memory, ring buffers for streaming, and specialized buffers for different parsing scenarios.

## BufferPool Struct

Thread-safe pool of reusable buffers.

```rust
pub struct BufferPool {
    buffers: Arc<Mutex<Vec<Vec<u8>>>>,
    buffer_size: usize,
    max_buffers: usize,
}
```

### Constructor

```rust
impl BufferPool {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffers: Arc::new(Mutex::new(Vec::new())),
            buffer_size,
            max_buffers: 64,
        }
    }
    
    pub fn with_capacity(buffer_size: usize, max_buffers: usize) -> Self {
        Self {
            buffers: Arc::new(Mutex::new(Vec::with_capacity(max_buffers))),
            buffer_size,
            max_buffers,
        }
    }
}
```

### Methods

#### get

Acquire a buffer from the pool.

```rust
pub fn get(&self) -> PooledBuffer {
    let mut buffers = self.buffers.lock().unwrap();
    
    let buffer = if let Some(buf) = buffers.pop() {
        buf
    } else {
        Vec::with_capacity(self.buffer_size)
    };
    
    PooledBuffer {
        buffer,
        pool: Arc::clone(&self.buffers),
    }
}
```

#### clear

Clear all buffers in the pool.

```rust
pub fn clear(&self) {
    let mut buffers = self.buffers.lock().unwrap();
    buffers.clear();
}
```

## PooledBuffer

A buffer that returns to the pool when dropped.

```rust
pub struct PooledBuffer {
    buffer: Option<Vec<u8>>,
    pool: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(mut buffer) = self.buffer.take() {
            buffer.clear();
            
            let mut pool = self.pool.lock().unwrap();
            if pool.len() < pool.capacity() {
                pool.push(buffer);
            }
        }
    }
}

impl Deref for PooledBuffer {
    type Target = Vec<u8>;
    
    fn deref(&self) -> &Self::Target {
        self.buffer.as_ref().unwrap()
    }
}
```

## RingBuffer

Circular buffer for streaming operations.

```rust
pub struct RingBuffer {
    buffer: Vec<u8>,
    capacity: usize,
    read_pos: usize,
    write_pos: usize,
    size: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0; capacity],
            capacity,
            read_pos: 0,
            write_pos: 0,
            size: 0,
        }
    }
    
    pub fn write(&mut self, data: &[u8]) -> usize {
        let available = self.capacity - self.size;
        let to_write = data.len().min(available);
        
        for i in 0..to_write {
            self.buffer[self.write_pos] = data[i];
            self.write_pos = (self.write_pos + 1) % self.capacity;
        }
        
        self.size += to_write;
        to_write
    }
    
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let to_read = buf.len().min(self.size);
        
        for i in 0..to_read {
            buf[i] = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.capacity;
        }
        
        self.size -= to_read;
        to_read
    }
    
    pub fn available(&self) -> usize {
        self.size
    }
    
    pub fn remaining(&self) -> usize {
        self.capacity - self.size
    }
}
```

## LineBuffer

Specialized buffer for line-oriented parsing.

```rust
pub struct LineBuffer {
    buffer: Vec<u8>,
    position: usize,
    filled: usize,
}

impl LineBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0; capacity],
            position: 0,
            filled: 0,
        }
    }
    
    pub fn fill<R: Read>(&mut self, reader: &mut R) -> io::Result<usize> {
        // Shift remaining data to beginning
        if self.position > 0 {
            self.buffer.copy_within(self.position..self.filled, 0);
            self.filled -= self.position;
            self.position = 0;
        }
        
        // Fill from reader
        let read = reader.read(&mut self.buffer[self.filled..])?;
        self.filled += read;
        Ok(read)
    }
    
    pub fn read_line(&mut self) -> Option<&[u8]> {
        if let Some(newline_pos) = memchr(b'\n', &self.buffer[self.position..self.filled]) {
            let line_start = self.position;
            let line_end = self.position + newline_pos;
            self.position = line_end + 1;
            Some(&self.buffer[line_start..line_end])
        } else {
            None
        }
    }
    
    pub fn consume(&mut self, amount: usize) {
        self.position = (self.position + amount).min(self.filled);
    }
    
    pub fn remaining(&self) -> &[u8] {
        &self.buffer[self.position..self.filled]
    }
}
```

## RecordBuffer

Buffer optimized for FASTQ records.

```rust
pub struct RecordBuffer {
    records: Vec<FastqRecord>,
    capacity: usize,
}

impl RecordBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            records: Vec::with_capacity(capacity),
            capacity,
        }
    }
    
    pub fn push(&mut self, record: FastqRecord) -> bool {
        if self.records.len() < self.capacity {
            self.records.push(record);
            true
        } else {
            false
        }
    }
    
    pub fn is_full(&self) -> bool {
        self.records.len() >= self.capacity
    }
    
    pub fn drain(&mut self) -> Vec<FastqRecord> {
        std::mem::replace(&mut self.records, Vec::with_capacity(self.capacity))
    }
    
    pub fn clear(&mut self) {
        self.records.clear();
    }
    
    pub fn len(&self) -> usize {
        self.records.len()
    }
}
```

## DoubleBuffer

Double buffering for concurrent read/write.

```rust
pub struct DoubleBuffer<T> {
    buffers: [Vec<T>; 2],
    active: AtomicUsize,
}

impl<T> DoubleBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffers: [
                Vec::with_capacity(capacity),
                Vec::with_capacity(capacity),
            ],
            active: AtomicUsize::new(0),
        }
    }
    
    pub fn get_write_buffer(&mut self) -> &mut Vec<T> {
        let index = self.active.load(Ordering::Acquire);
        &mut self.buffers[index]
    }
    
    pub fn swap(&self) {
        let current = self.active.load(Ordering::Acquire);
        let next = 1 - current;
        self.active.store(next, Ordering::Release);
    }
    
    pub fn get_read_buffer(&self) -> &Vec<T> {
        let index = 1 - self.active.load(Ordering::Acquire);
        &self.buffers[index]
    }
}
```

## Memory Allocation Strategies

### Pre-allocation

Pre-allocate buffers to avoid runtime allocations.

```rust
pub struct PreallocatedBuffers {
    buffers: Vec<Vec<u8>>,
    next: usize,
}

impl PreallocatedBuffers {
    pub fn new(count: usize, size: usize) -> Self {
        let buffers = (0..count)
            .map(|_| Vec::with_capacity(size))
            .collect();
        
        Self {
            buffers,
            next: 0,
        }
    }
    
    pub fn get_next(&mut self) -> &mut Vec<u8> {
        let buffer = &mut self.buffers[self.next];
        self.next = (self.next + 1) % self.buffers.len();
        buffer.clear();
        buffer
    }
}
```

### Arena Allocator

Arena allocation for temporary buffers.

```rust
pub struct Arena {
    buffer: Vec<u8>,
    position: usize,
}

impl Arena {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0; capacity],
            position: 0,
        }
    }
    
    pub fn allocate(&mut self, size: usize) -> Option<&mut [u8]> {
        if self.position + size <= self.buffer.len() {
            let start = self.position;
            self.position += size;
            Some(&mut self.buffer[start..self.position])
        } else {
            None
        }
    }
    
    pub fn reset(&mut self) {
        self.position = 0;
    }
}
```

## Buffer Utilities

### Buffer Sizing

Calculate optimal buffer sizes.

```rust
pub fn optimal_buffer_size(file_size: usize) -> usize {
    const MIN_BUFFER: usize = 8 * 1024;      // 8 KB
    const MAX_BUFFER: usize = 64 * 1024;     // 64 KB
    
    let suggested = (file_size / 100).max(MIN_BUFFER).min(MAX_BUFFER);
    
    // Round to page size
    let page_size = 4096;
    ((suggested + page_size - 1) / page_size) * page_size
}
```

### Memory Statistics

Track buffer usage.

```rust
pub struct BufferStats {
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
    total_bytes: AtomicUsize,
    peak_bytes: AtomicUsize,
}

impl BufferStats {
    pub fn record_allocation(&self, size: usize) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        let total = self.total_bytes.fetch_add(size, Ordering::Relaxed) + size;
        
        let mut peak = self.peak_bytes.load(Ordering::Relaxed);
        while total > peak {
            match self.peak_bytes.compare_exchange_weak(
                peak, total,
                Ordering::Relaxed,
                Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }
    }
    
    pub fn record_deallocation(&self, size: usize) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_sub(size, Ordering::Relaxed);
    }
}
```

## Examples

### Using Buffer Pool

```rust
use fastq_parser::buffer::BufferPool;

fn process_with_pool() -> Result<()> {
    let pool = BufferPool::new(4096);
    
    for i in 0..1000 {
        let mut buffer = pool.get();
        
        // Use buffer for parsing
        buffer.extend_from_slice(b"@seq\nACGT\n+\nIIII\n");
        
        // Process data
        process_data(&buffer);
        
        // Buffer automatically returns to pool when dropped
    }
    
    Ok(())
}
```

### Streaming with Ring Buffer

```rust
use fastq_parser::buffer::RingBuffer;

fn stream_parse<R: Read>(reader: &mut R) -> Result<()> {
    let mut ring = RingBuffer::new(8192);
    let mut temp = vec![0; 4096];
    
    loop {
        let read = reader.read(&mut temp)?;
        if read == 0 { break; }
        
        ring.write(&temp[..read]);
        
        while ring.available() >= 4 {
            // Process available data
            let mut record_buf = vec![0; ring.available()];
            ring.read(&mut record_buf);
            process_record(&record_buf)?;
        }
    }
    
    Ok(())
}
```

### Double Buffering for I/O

```rust
use fastq_parser::buffer::DoubleBuffer;
use std::thread;

fn double_buffer_io() -> Result<()> {
    let mut buffers = DoubleBuffer::<FastqRecord>::new(1000);
    
    // Reader thread
    thread::spawn(move || {
        loop {
            let write_buf = buffers.get_write_buffer();
            // Fill buffer with records
            fill_records(write_buf);
            buffers.swap();
        }
    });
    
    // Processor thread
    loop {
        let read_buf = buffers.get_read_buffer();
        for record in read_buf {
            process_record(record);
        }
    }
}
```

## Performance Considerations

1. **Pool Size**: Balance between memory usage and allocation frequency
2. **Buffer Size**: Larger buffers reduce syscalls but increase memory
3. **Pre-allocation**: Reduces runtime allocations but increases startup time
4. **Arena Reset**: Frequent resets vs larger arena size
5. **Double Buffering**: Reduces I/O wait time in concurrent scenarios

## See Also

- [Reader Module](./reader.md) - Uses buffers for file I/O
- [Parser Module](./parser.md) - Buffer-based parsing
- [Parallel Module](./parallel.md) - Buffer management in parallel context