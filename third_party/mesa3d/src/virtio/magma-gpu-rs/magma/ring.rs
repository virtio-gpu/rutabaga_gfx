// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::mem::size_of;
use std::sync::atomic::{AtomicU32, Ordering};

use magma_gpu::util::Error as MagmaGpuError;
use magma_gpu::util::MappedRegion;
use magma_gpu::util::MemoryMapping;
use magma_gpu::util::Result as MagmaGpuResult;

pub const MAGMA_RING_MAGIC: u32 = 0x4d41474d; // "MAGM"
pub const RING_STATUS_ACTIVE: u32 = 0;
pub const RING_STATUS_SPINNING: u32 = 1;
pub const RING_STATUS_ASLEEP: u32 = 2;

/// Shared control header placed at the beginning of the ring buffer memory mapping.
/// Padded to 64 bytes so the command ring data starts on a clean cacheline boundary.
#[repr(C)]
pub struct MagmaRingControl {
    /// Sentinel word at offset 0, watched by AtomicMemorySentinel (futex).
    pub sentinel: AtomicU32,
    /// Magic identifier for verification ("MAGM").
    pub magic: u32,
    /// Size of the circular data region in bytes.
    pub ring_size: u32,
    /// Read offset updated by host consumer (Release store, Acquire load by guest).
    pub read_offset: AtomicU32,
    /// Write offset updated by guest producer (Release store, Acquire load by host).
    pub write_offset: AtomicU32,
    /// Current thread status: 0 = ACTIVE, 1 = SPINNING, 2 = ASLEEP.
    pub status: AtomicU32,
    /// Latest seqno submitted by guest producer.
    pub submitted_seqno: AtomicU32,
    /// Latest seqno completed by host consumer.
    pub completed_seqno: AtomicU32,
    /// Padding to align header to 64 bytes.
    pub _pad: [u32; 8],
}

const _: () = assert!(size_of::<MagmaRingControl>() == 64);

pub const CONTROL_HEADER_SIZE: usize = size_of::<MagmaRingControl>();

pub struct MagmaRingBuffer {
    _mapping: Option<MemoryMapping>,
    control: *mut MagmaRingControl,
    data_ptr: *mut u8,
    data_size: usize,
}

// SAFETY: MagmaRingBuffer operates on thread-safe atomic control pointers
// and raw ring memory mapped from a shared descriptor.
unsafe impl Send for MagmaRingBuffer {}
unsafe impl Sync for MagmaRingBuffer {}

impl MagmaRingBuffer {
    pub fn new(mapping: MemoryMapping) -> MagmaGpuResult<Self> {
        let base_ptr = mapping.as_ptr();
        let size = mapping.size();
        let mut ring = unsafe { Self::from_raw_parts(base_ptr, size)? };
        ring._mapping = Some(mapping);
        Ok(ring)
    }

    /// # Safety
    ///
    /// The caller must ensure that `base_ptr` points to a valid, properly aligned,
    /// and accessible region of memory of at least `total_size` bytes for the lifetime
    /// of the returned `MagmaRingBuffer`.
    pub unsafe fn from_raw_parts(base_ptr: *mut u8, total_size: usize) -> MagmaGpuResult<Self> {
        if total_size <= CONTROL_HEADER_SIZE {
            return Err(MagmaGpuError::WithContext(
                "buffer too small for ring control",
            ));
        }

        let control = base_ptr as *mut MagmaRingControl;
        let data_ptr = unsafe { base_ptr.add(CONTROL_HEADER_SIZE) };
        let data_size = total_size - CONTROL_HEADER_SIZE;

        // Initialize header if uninitialized
        let ctrl_ref = unsafe { &*control };
        if ctrl_ref.magic != MAGMA_RING_MAGIC {
            ctrl_ref.sentinel.store(0, Ordering::Relaxed);
            unsafe {
                (*control).magic = MAGMA_RING_MAGIC;
                (*control).ring_size = data_size as u32;
            }
            ctrl_ref.read_offset.store(0, Ordering::Relaxed);
            ctrl_ref.write_offset.store(0, Ordering::Relaxed);
            ctrl_ref.status.store(RING_STATUS_ACTIVE, Ordering::Relaxed);
            ctrl_ref.submitted_seqno.store(0, Ordering::Relaxed);
            ctrl_ref.completed_seqno.store(0, Ordering::Relaxed);
        }

        Ok(Self {
            _mapping: None,
            control,
            data_ptr,
            data_size,
        })
    }

    #[inline]
    pub fn control(&self) -> &MagmaRingControl {
        unsafe { &*self.control }
    }

    #[inline]
    pub fn has_data(&self) -> bool {
        let ctrl = self.control();
        ctrl.read_offset.load(Ordering::Relaxed) != ctrl.write_offset.load(Ordering::Acquire)
    }

    #[inline]
    pub fn status(&self) -> u32 {
        self.control().status.load(Ordering::Acquire)
    }

    #[inline]
    pub fn set_status(&self, status: u32) {
        self.control().status.store(status, Ordering::Release);
    }

    #[inline]
    pub fn submitted_seqno(&self) -> u32 {
        self.control().submitted_seqno.load(Ordering::Acquire)
    }

    #[inline]
    pub fn set_submitted_seqno(&self, seqno: u32) {
        self.control()
            .submitted_seqno
            .store(seqno, Ordering::Release);
    }

    #[inline]
    pub fn completed_seqno(&self) -> u32 {
        self.control().completed_seqno.load(Ordering::Acquire)
    }

    #[inline]
    pub fn set_completed_seqno(&self, seqno: u32) {
        self.control()
            .completed_seqno
            .store(seqno, Ordering::Release);
    }

    #[inline]
    #[allow(dead_code)]
    pub fn load_sentinel(&self) -> u32 {
        self.control().sentinel.load(Ordering::Acquire)
    }

    /// Write a command into the circular buffer.
    pub fn write(&self, bytes: &[u8]) -> MagmaGpuResult<usize> {
        let ctrl = self.control();
        let ring_size = self.data_size;
        let write_len = bytes.len();

        if write_len == 0 {
            return Ok(0);
        }
        if write_len >= ring_size {
            return Err(MagmaGpuError::WithContext("command exceeds ring size"));
        }

        let mut spin_count = 0;
        loop {
            let read = ctrl.read_offset.load(Ordering::Acquire) as usize;
            let write = ctrl.write_offset.load(Ordering::Relaxed) as usize;

            let available = if write >= read {
                ring_size - 1 - (write - read)
            } else {
                read - write - 1
            };

            if write_len <= available {
                if write + write_len <= ring_size {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            bytes.as_ptr(),
                            self.data_ptr.add(write),
                            write_len,
                        );
                    }
                    let new_write = (write + write_len) % ring_size;
                    ctrl.write_offset.store(new_write as u32, Ordering::Release);
                } else {
                    let first_chunk = ring_size - write;
                    let second_chunk = write_len - first_chunk;
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            bytes.as_ptr(),
                            self.data_ptr.add(write),
                            first_chunk,
                        );
                        std::ptr::copy_nonoverlapping(
                            bytes.as_ptr().add(first_chunk),
                            self.data_ptr,
                            second_chunk,
                        );
                    }
                    ctrl.write_offset
                        .store(second_chunk as u32, Ordering::Release);
                }
                return Ok(write_len);
            }

            spin_count += 1;
            if spin_count > 100_000 {
                return Err(MagmaGpuError::WithContext("ring buffer full"));
            }
            std::hint::spin_loop();
        }
    }

    /// Read available contiguous slice(s) of commands from the ring.
    pub fn read_available<F>(&self, mut consumer: F) -> usize
    where
        F: FnMut(&[u8]) -> usize,
    {
        let ctrl = self.control();
        let read = ctrl.read_offset.load(Ordering::Relaxed) as usize;
        let write = ctrl.write_offset.load(Ordering::Acquire) as usize;

        if read == write || self.data_size == 0 {
            return 0;
        }

        let ring_size = self.data_size;
        let mut total_consumed = 0;

        if write > read {
            let slice =
                unsafe { std::slice::from_raw_parts(self.data_ptr.add(read), write - read) };
            let consumed = consumer(slice);
            if consumed > 0 {
                let new_read = (read + consumed) % ring_size;
                ctrl.read_offset.store(new_read as u32, Ordering::Release);
                total_consumed += consumed;
            }
        } else {
            // Wrapped: first read from `read` to end of data area
            let slice_end =
                unsafe { std::slice::from_raw_parts(self.data_ptr.add(read), ring_size - read) };
            let consumed_end = consumer(slice_end);
            let mut current_read = read;
            if consumed_end > 0 {
                current_read = (read + consumed_end) % ring_size;
                ctrl.read_offset
                    .store(current_read as u32, Ordering::Release);
                total_consumed += consumed_end;
            }

            // If consumed all the way to the end, process from 0 to `write`
            if current_read == 0 && write > 0 {
                let slice_start = unsafe { std::slice::from_raw_parts(self.data_ptr, write) };
                let consumed_start = consumer(slice_start);
                if consumed_start > 0 {
                    ctrl.read_offset
                        .store(consumed_start as u32, Ordering::Release);
                    total_consumed += consumed_start;
                }
            } else if current_read > 0 && write > 0 {
                // Command was split across boundary: assemble into scratch buffer
                let tail_len = ring_size - current_read;
                let head_len = write.min(4096 - tail_len);
                if tail_len + head_len <= 4096 {
                    let mut scratch = [0u8; 4096];
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            self.data_ptr.add(current_read),
                            scratch.as_mut_ptr(),
                            tail_len,
                        );
                        std::ptr::copy_nonoverlapping(
                            self.data_ptr,
                            scratch.as_mut_ptr().add(tail_len),
                            head_len,
                        );
                    }
                    let consumed_scratch = consumer(&scratch[..tail_len + head_len]);
                    if consumed_scratch > tail_len {
                        let head_consumed = consumed_scratch - tail_len;
                        ctrl.read_offset
                            .store(head_consumed as u32, Ordering::Release);
                        total_consumed += consumed_scratch;

                        if write > head_consumed {
                            let slice_start = unsafe {
                                std::slice::from_raw_parts(
                                    self.data_ptr.add(head_consumed),
                                    write - head_consumed,
                                )
                            };
                            let consumed_more = consumer(slice_start);
                            if consumed_more > 0 {
                                ctrl.read_offset.store(
                                    (head_consumed + consumed_more) as u32,
                                    Ordering::Release,
                                );
                                total_consumed += consumed_more;
                            }
                        }
                    }
                }
            }
        }

        total_consumed
    }
}
