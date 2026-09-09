// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

#![allow(dead_code)]

use std::os::fd::AsFd;
use std::os::fd::BorrowedFd;
use std::sync::Arc;

use log::error;

use magma_gpu::log_status;
use magma_gpu::util::AsRawDescriptor;
use magma_gpu::util::Error as MagmaGpuError;
use magma_gpu::util::FromRawDescriptor;
use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MappedRegion;
use magma_gpu::util::MemoryMapping;
use magma_gpu::util::OwnedDescriptor;
use magma_gpu::util::Result as MagmaGpuResult;
use magma_gpu::util::MAGMA_GPU_HANDLE_TYPE_SIGNAL_SYNC_FD;

use crate::ioctl_readwrite;
use crate::ioctl_write_ptr;

use crate::traits::AddressSpace;
use crate::traits::AsVirtGpu;
use crate::traits::Buffer;
use crate::traits::Device;
use crate::traits::GenericAddressSpace;
use crate::traits::GenericBuffer;
use crate::traits::GenericDevice;
use crate::traits::GenericPhysicalDevice;
use crate::traits::GenericQueue;
use crate::traits::GenericSyncObject;
use crate::traits::PhysicalDevice;
use crate::traits::Queue;
use crate::traits::SyncObject;

use crate::magma_defines::MagmaCreateBufferInfo;
use crate::magma_defines::MagmaCreateQueueInfo;
use crate::magma_defines::MagmaCreateSyncObjInfo;
use crate::magma_defines::MagmaError;
use crate::magma_defines::MagmaHeapBudget;
use crate::magma_defines::MagmaImportHandleInfo;
use crate::magma_defines::MagmaMappedMemoryRange;
use crate::magma_defines::MagmaMemoryProperties;
use crate::magma_defines::MagmaPhysicalDeviceInfo;
use crate::magma_defines::MagmaQueueFamilyProperties;
use crate::magma_defines::MagmaQueueFlags;
use crate::magma_defines::MagmaResult;
use crate::magma_defines::MagmaSubmitInfo;
use crate::magma_defines::MAGMA_HEAP_CPU_VISIBLE_BIT;
use crate::magma_defines::MAGMA_HEAP_DEVICE_LOCAL_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT;
use crate::protocol::MagmaGpuMapFlags;

use crate::sys::linux::bindings::kgsl_bindings::*;
use crate::sys::linux::PlatformDevice;
use crate::sys::linux::PlatformPhysicalDevice;

const KGSL_IOC_TYPE: u32 = 0x09;

ioctl_readwrite!(
    kgsl_ioctl_device_getproperty,
    KGSL_IOC_TYPE,
    0x02,
    kgsl_device_getproperty
);

ioctl_readwrite!(
    kgsl_ioctl_drawctxt_create,
    KGSL_IOC_TYPE,
    0x13,
    kgsl_drawctxt_create
);

ioctl_write_ptr!(
    kgsl_ioctl_drawctxt_destroy,
    KGSL_IOC_TYPE,
    0x14,
    kgsl_drawctxt_destroy
);

ioctl_readwrite!(
    kgsl_ioctl_gpumem_alloc_id,
    KGSL_IOC_TYPE,
    0x34,
    kgsl_gpumem_alloc_id
);

ioctl_readwrite!(
    kgsl_ioctl_gpumem_free_id,
    KGSL_IOC_TYPE,
    0x35,
    kgsl_gpumem_free_id
);

ioctl_readwrite!(
    kgsl_ioctl_gpuobj_alloc,
    KGSL_IOC_TYPE,
    0x45,
    kgsl_gpuobj_alloc
);

ioctl_write_ptr!(
    kgsl_ioctl_gpuobj_free,
    KGSL_IOC_TYPE,
    0x46,
    kgsl_gpuobj_free
);

ioctl_readwrite!(
    kgsl_ioctl_gpuobj_info,
    KGSL_IOC_TYPE,
    0x47,
    kgsl_gpuobj_info
);

ioctl_readwrite!(
    kgsl_ioctl_gpuobj_import,
    KGSL_IOC_TYPE,
    0x48,
    kgsl_gpuobj_import
);

ioctl_readwrite!(
    kgsl_ioctl_gpu_command,
    KGSL_IOC_TYPE,
    0x4A,
    kgsl_gpu_command
);

ioctl_readwrite!(
    kgsl_ioctl_gpumem_bind_ranges,
    KGSL_IOC_TYPE,
    0x56,
    kgsl_gpumem_bind_ranges
);

ioctl_readwrite!(
    kgsl_ioctl_syncsource_create,
    KGSL_IOC_TYPE,
    0x40,
    kgsl_syncsource_create
);

ioctl_readwrite!(
    kgsl_ioctl_syncsource_destroy,
    KGSL_IOC_TYPE,
    0x41,
    kgsl_syncsource_destroy
);

ioctl_readwrite!(
    kgsl_ioctl_syncsource_create_fence,
    KGSL_IOC_TYPE,
    0x42,
    kgsl_syncsource_create_fence
);

ioctl_readwrite!(
    kgsl_ioctl_syncsource_signal_fence,
    KGSL_IOC_TYPE,
    0x43,
    kgsl_syncsource_signal_fence
);

pub struct KgslSyncObject {
    physical_device: Arc<dyn PhysicalDevice>,
    syncsource_id: u32,
    fence_fd: i32,
}

impl Drop for KgslSyncObject {
    fn drop(&mut self) {
        if self.fence_fd >= 0 {
            unsafe {
                libc::close(self.fence_fd);
            }
        }
        if self.syncsource_id != 0 {
            if let Some(fd) = self.physical_device.as_fd() {
                let mut destroy = kgsl_syncsource_destroy {
                    id: self.syncsource_id,
                    ..Default::default()
                };
                unsafe {
                    let _ = kgsl_ioctl_syncsource_destroy(fd, &mut destroy);
                }
            }
        }
    }
}

impl GenericSyncObject for KgslSyncObject {
    fn wait(&self, timeout_ns: u64) -> MagmaResult<()> {
        if self.fence_fd < 0 {
            return Ok(());
        }
        let timeout_ms = if timeout_ns == u64::MAX {
            -1
        } else {
            (timeout_ns / 1_000_000).min(i32::MAX as u64) as i32
        };
        let mut pfd = libc::pollfd {
            fd: self.fence_fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if ret < 0 {
            Err(MagmaError::InternalError)
        } else if ret == 0 {
            Err(MagmaError::TimedOut)
        } else {
            Ok(())
        }
    }

    fn signal(&self) -> MagmaGpuResult<()> {
        let fd = self
            .physical_device
            .as_fd()
            .ok_or(MagmaGpuError::Unsupported)?;
        let mut sig = kgsl_syncsource_signal_fence {
            id: self.syncsource_id,
            fence_fd: self.fence_fd,
            ..Default::default()
        };
        unsafe {
            kgsl_ioctl_syncsource_signal_fence(fd, &mut sig)?;
        }
        Ok(())
    }

    fn export_fence(&self) -> MagmaGpuResult<MagmaGpuHandle> {
        if self.fence_fd < 0 {
            return Err(MagmaGpuError::Unsupported);
        }
        let dup_fd = unsafe { libc::dup(self.fence_fd) };
        if dup_fd < 0 {
            return Err(MagmaGpuError::WithContext("dup failed"));
        }
        let descriptor = unsafe { OwnedDescriptor::from_raw_descriptor(dup_fd) };
        Ok(MagmaGpuHandle {
            os_handle: descriptor,
            handle_type: MAGMA_GPU_HANDLE_TYPE_SIGNAL_SYNC_FD,
        })
    }

    fn as_raw_handle(&self) -> Option<u32> {
        if self.fence_fd >= 0 {
            Some(self.fence_fd as u32)
        } else {
            None
        }
    }
}

impl SyncObject for KgslSyncObject {}

const VBO_SIZE_LADDER: [u64; 6] = [
    0x100_0000_0000, // 1 TiB
    0x40_0000_0000,  // 256 GiB
    0x20_0000_0000,  // 128 GiB
    0x10_0000_0000,  // 64 GiB
    0x4_0000_0000,   // 16 GiB
    0x1_0000_0000,   // 4 GiB
];

#[derive(Debug)]
#[allow(dead_code)]
pub struct KgslPhysicalDevice {
    descriptor: OwnedDescriptor,
    chip_id: u64,
    gpu_id: u32,
    gmem_size: u64,
}

#[allow(dead_code)]
impl KgslPhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> Self {
        let (chip_id, gpu_id, gmem_size) = if let Ok(info) = Self::get_devinfo(descriptor.as_fd()) {
            let cid = info.chip_id as u64;
            let gid = if info.gpu_id != 0 {
                info.gpu_id
            } else {
                (((cid >> 24) & 0xff) * 100 + ((cid >> 16) & 0xff) * 10 + ((cid >> 8) & 0xff))
                    as u32
            };
            (cid, gid, info.gmem_sizebytes)
        } else {
            (0, 0, 0)
        };

        Self {
            descriptor,
            chip_id,
            gpu_id,
            gmem_size,
        }
    }

    fn get_devinfo(fd: BorrowedFd<'_>) -> MagmaGpuResult<kgsl_devinfo> {
        let mut info: kgsl_devinfo = Default::default();
        let mut getprop = kgsl_device_getproperty {
            type_: KGSL_PROP_DEVICE_INFO,
            value: &mut info as *mut _ as *mut std::os::raw::c_void,
            sizebytes: std::mem::size_of::<kgsl_devinfo>() as u64,
        };
        // SAFETY: fd is valid and info is properly aligned and sized.
        unsafe {
            kgsl_ioctl_device_getproperty(fd, &mut getprop)?;
        }
        Ok(info)
    }

    pub fn chip_id(&self) -> u64 {
        self.chip_id
    }

    pub fn gpu_id(&self) -> u32 {
        self.gpu_id
    }

    pub fn gmem_size(&self) -> u64 {
        self.gmem_size
    }
}

impl PlatformPhysicalDevice for KgslPhysicalDevice {
    fn as_descriptor(&self) -> Option<&OwnedDescriptor> {
        Some(&self.descriptor)
    }

    fn cpu_map(&self, offset: u64, size: usize) -> MagmaGpuResult<MemoryMapping> {
        let desc = self.as_descriptor().ok_or(MagmaGpuError::Unsupported)?;
        MemoryMapping::from_offset(desc, offset.try_into()?, size)
    }

    fn export(&self, _gem_handle: u32) -> MagmaGpuResult<MagmaGpuHandle> {
        Err(MagmaGpuError::Unsupported)
    }

    fn import(&self, handle: MagmaGpuHandle) -> MagmaGpuResult<u32> {
        let fd = self.as_fd().ok_or(MagmaGpuError::Unsupported)?;
        let mut import_dmabuf = kgsl_gpuobj_import_dma_buf {
            fd: handle.os_handle.as_raw_descriptor(),
        };
        let mut req = kgsl_gpuobj_import {
            priv_: &mut import_dmabuf as *mut _ as u64,
            priv_len: std::mem::size_of::<kgsl_gpuobj_import_dma_buf>() as u64,
            flags: 0,
            type_: KGSL_USER_MEM_TYPE_DMABUF,
            id: 0,
        };
        // SAFETY: fd is valid and req references valid import payload.
        unsafe {
            kgsl_ioctl_gpuobj_import(fd, &mut req)?;
        }
        Ok(req.id)
    }

    fn close(&self, gem_handle: u32) {
        if let Some(fd) = self.as_fd() {
            let mut req = kgsl_gpumem_free_id {
                id: gem_handle,
                __pad: 0,
            };
            // SAFETY: fd is valid and req is properly initialized.
            let result = unsafe { kgsl_ioctl_gpumem_free_id(fd, &mut req) };
            log_status!(result);
        }
    }
}

impl AsVirtGpu for KgslPhysicalDevice {}
impl PhysicalDevice for KgslPhysicalDevice {}
unsafe impl Send for KgslPhysicalDevice {}
unsafe impl Sync for KgslPhysicalDevice {}

impl GenericPhysicalDevice for KgslPhysicalDevice {
    fn create_device(
        self: Arc<Self>,
        _info: &MagmaPhysicalDeviceInfo,
    ) -> MagmaGpuResult<Arc<dyn Device>> {
        Ok(Arc::new(Kgsl::new(self)?))
    }

    fn query_memory_properties(&self) -> MagmaGpuResult<MagmaMemoryProperties> {
        let mut mem_props: MagmaMemoryProperties = Default::default();
        let heap_size = 4 * 1024 * 1024 * 1024; // 4 GiB default unified memory heap
        mem_props.add_heap(
            heap_size,
            MAGMA_HEAP_DEVICE_LOCAL_BIT | MAGMA_HEAP_CPU_VISIBLE_BIT,
        );
        mem_props.add_memory_type(
            MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT,
        );
        mem_props.add_memory_type(
            MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT,
        );
        mem_props.increment_heap_count();
        Ok(mem_props)
    }

    fn query_queue_family_properties(&self) -> MagmaGpuResult<Vec<MagmaQueueFamilyProperties>> {
        Ok(vec![MagmaQueueFamilyProperties::new(
            MagmaQueueFlags::Graphics | MagmaQueueFlags::Compute,
            1,
        )])
    }
}

pub struct Kgsl {
    physical_device: Arc<dyn PhysicalDevice>,
    mem_props: MagmaMemoryProperties,
}

impl Kgsl {
    pub fn new(physical_device: Arc<dyn PhysicalDevice>) -> MagmaGpuResult<Kgsl> {
        let mem_props = physical_device.query_memory_properties()?;
        Ok(Kgsl {
            physical_device,
            mem_props,
        })
    }
}

impl GenericDevice for Kgsl {
    fn get_memory_budget(&self, _heap_idx: u32) -> MagmaGpuResult<MagmaHeapBudget> {
        Err(MagmaGpuError::Unsupported)
    }

    fn create_address_space(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn AddressSpace>> {
        Ok(Arc::new(KgslAddressSpace::new(
            self.physical_device.clone(),
        )?))
    }

    fn create_queue(
        self: Arc<Self>,
        _address_space: &Arc<dyn AddressSpace>,
        _info: &MagmaCreateQueueInfo,
    ) -> MagmaGpuResult<Arc<dyn Queue>> {
        let mut create_req = kgsl_drawctxt_create {
            flags: KGSL_CONTEXT_SAVE_GMEM | KGSL_CONTEXT_NO_GMEM_ALLOC | KGSL_CONTEXT_PREAMBLE,
            drawctxt_id: 0,
        };
        // SAFETY: Underlying descriptor is valid.
        unsafe {
            kgsl_ioctl_drawctxt_create(self.physical_device.as_fd().unwrap(), &mut create_req)?;
        }
        Ok(Arc::new(KgslQueue {
            physical_device: self.physical_device.clone(),
            context_id: create_req.drawctxt_id,
        }))
    }

    fn create_buffer(
        self: Arc<Self>,
        create_info: &MagmaCreateBufferInfo,
    ) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let buf = KgslBuffer::new(self.physical_device.clone(), create_info, &self.mem_props)?;
        Ok(Arc::new(buf))
    }

    fn import(self: Arc<Self>, info: MagmaImportHandleInfo) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let id = self.physical_device.import(info.handle)?;
        let buf =
            KgslBuffer::from_existing(self.physical_device.clone(), id, info.size.try_into()?)?;
        Ok(Arc::new(buf))
    }

    fn create_sync_obj(
        self: Arc<Self>,
        _info: &MagmaCreateSyncObjInfo,
    ) -> MagmaGpuResult<Arc<dyn SyncObject>> {
        let fd = self
            .physical_device
            .as_fd()
            .ok_or(MagmaGpuError::Unsupported)?;
        let mut create = kgsl_syncsource_create {
            id: 0,
            ..Default::default()
        };
        unsafe {
            kgsl_ioctl_syncsource_create(fd, &mut create)?;
        }
        let mut create_fence = kgsl_syncsource_create_fence {
            id: create.id,
            fence_fd: -1,
            ..Default::default()
        };
        unsafe {
            kgsl_ioctl_syncsource_create_fence(fd, &mut create_fence)?;
        }
        Ok(Arc::new(KgslSyncObject {
            physical_device: self.physical_device.clone(),
            syncsource_id: create.id,
            fence_fd: create_fence.fence_fd,
        }))
    }

    fn import_sync_obj(
        self: Arc<Self>,
        info: MagmaImportHandleInfo,
    ) -> MagmaGpuResult<Arc<dyn SyncObject>> {
        let dup_fd = unsafe { libc::dup(info.handle.os_handle.as_raw_descriptor()) };
        if dup_fd < 0 {
            return Err(MagmaGpuError::WithContext("dup failed"));
        }
        Ok(Arc::new(KgslSyncObject {
            physical_device: self.physical_device.clone(),
            syncsource_id: 0,
            fence_fd: dup_fd,
        }))
    }
}

impl PlatformDevice for Kgsl {}
impl Device for Kgsl {}

pub struct KgslBuffer {
    physical_device: Arc<dyn PhysicalDevice>,
    id: u32,
    #[allow(dead_code)]
    gpuaddr: u64,
    size: usize,
}

impl KgslBuffer {
    pub fn new(
        physical_device: Arc<dyn PhysicalDevice>,
        create_info: &MagmaCreateBufferInfo,
        mem_props: &MagmaMemoryProperties,
    ) -> MagmaGpuResult<KgslBuffer> {
        let memory_type = mem_props.get_memory_type(create_info.memory_type_idx);
        let flags = if memory_type.is_cached() {
            (KGSL_CACHEMODE_WRITEBACK << KGSL_CACHEMODE_SHIFT) | KGSL_MEMFLAGS_IOCOHERENT
        } else {
            KGSL_CACHEMODE_WRITECOMBINE << KGSL_CACHEMODE_SHIFT
        };

        let mut req = kgsl_gpumem_alloc_id {
            size: create_info.size,
            flags,
            ..Default::default()
        };

        // SAFETY: Underlying descriptor is valid.
        unsafe {
            kgsl_ioctl_gpumem_alloc_id(physical_device.as_fd().unwrap(), &mut req)?;
        }

        Ok(KgslBuffer {
            physical_device,
            id: req.id,
            gpuaddr: req.gpuaddr,
            size: create_info.size.try_into()?,
        })
    }

    pub fn from_existing(
        physical_device: Arc<dyn PhysicalDevice>,
        id: u32,
        size: usize,
    ) -> MagmaGpuResult<KgslBuffer> {
        let mut info_req = kgsl_gpuobj_info {
            id,
            ..Default::default()
        };
        let gpuaddr = unsafe {
            if kgsl_ioctl_gpuobj_info(physical_device.as_fd().unwrap(), &mut info_req).is_ok() {
                info_req.gpuaddr
            } else {
                0
            }
        };

        Ok(KgslBuffer {
            physical_device,
            id,
            gpuaddr,
            size,
        })
    }
}

impl GenericBuffer for KgslBuffer {
    fn map(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn MappedRegion>> {
        let offset = (self.id as u64) << 12;
        let mapping = self.physical_device.cpu_map(offset, self.size)?;
        Ok(Arc::new(mapping))
    }

    fn export(&self) -> MagmaGpuResult<MagmaGpuHandle> {
        self.physical_device.export(self.id)
    }

    fn invalidate(
        &self,
        _sync_flags: u64,
        _ranges: &[MagmaMappedMemoryRange],
    ) -> MagmaGpuResult<()> {
        Ok(())
    }

    fn flush(&self, _sync_flags: u64, _ranges: &[MagmaMappedMemoryRange]) -> MagmaGpuResult<()> {
        Ok(())
    }

    fn as_gem_handle(&self) -> Option<u32> {
        Some(self.id)
    }
}

impl Drop for KgslBuffer {
    fn drop(&mut self) {
        self.physical_device.close(self.id);
    }
}

impl Buffer for KgslBuffer {}

pub struct KgslAddressSpace {
    physical_device: Arc<dyn PhysicalDevice>,
    vbo_id: u32,
    vbo_base: u64,
    vbo_size: u64,
}

impl KgslAddressSpace {
    pub fn new(physical_device: Arc<dyn PhysicalDevice>) -> MagmaGpuResult<Self> {
        let fd = physical_device.as_fd().ok_or(MagmaGpuError::Unsupported)?;

        let mut vbo_id = 0;
        let mut vbo_base = 0;
        let mut vbo_size = 0;

        for &size in &VBO_SIZE_LADDER {
            let mut req = kgsl_gpuobj_alloc {
                size,
                flags: KGSL_MEMFLAGS_VBO | KGSL_MEMFLAGS_VBO_NO_MAP_ZERO,
                ..Default::default()
            };

            // SAFETY: fd is valid and req is properly initialized.
            if unsafe { kgsl_ioctl_gpuobj_alloc(fd, &mut req).is_ok() } {
                let mut info = kgsl_gpuobj_info {
                    id: req.id,
                    ..Default::default()
                };
                // SAFETY: fd is valid and info is properly initialized.
                if unsafe { kgsl_ioctl_gpuobj_info(fd, &mut info).is_ok() } {
                    vbo_id = req.id;
                    vbo_base = info.gpuaddr;
                    vbo_size = size;
                    break;
                } else {
                    let mut free_req = kgsl_gpumem_free_id {
                        id: req.id,
                        __pad: 0,
                    };
                    // SAFETY: fd is valid and free_req is properly initialized.
                    unsafe {
                        let _ = kgsl_ioctl_gpumem_free_id(fd, &mut free_req);
                    }
                }
            }
        }

        Ok(KgslAddressSpace {
            physical_device,
            vbo_id,
            vbo_base,
            vbo_size,
        })
    }

    pub fn compute_target_offset(&self, gpu_va: u64) -> u64 {
        if gpu_va >= self.vbo_base {
            gpu_va - self.vbo_base
        } else {
            gpu_va
        }
    }
}

impl Drop for KgslAddressSpace {
    fn drop(&mut self) {
        if self.vbo_id != 0 {
            self.physical_device.close(self.vbo_id);
        }
    }
}

impl GenericAddressSpace for KgslAddressSpace {
    fn as_vm_id(&self) -> Option<u32> {
        if self.vbo_id != 0 {
            Some(self.vbo_id)
        } else {
            None
        }
    }

    fn map_buffer_gpu(
        &self,
        buffer: &Arc<dyn Buffer>,
        buffer_offset: u64,
        gpu_va: u64,
        size: u64,
        _flags: MagmaGpuMapFlags,
    ) -> MagmaGpuResult<()> {
        let child_id = buffer.as_gem_handle().ok_or(MagmaGpuError::Unsupported)?;

        if self.vbo_id == 0 {
            return Ok(());
        }

        let target_offset = self.compute_target_offset(gpu_va);
        if target_offset
            .checked_add(size)
            .is_none_or(|end| end > self.vbo_size)
        {
            return Err(MagmaGpuError::Unsupported);
        }

        let range = kgsl_gpumem_bind_range {
            child_offset: buffer_offset,
            target_offset,
            length: size,
            child_id,
            op: KGSL_GPUMEM_RANGE_OP_BIND,
        };

        let mut req = kgsl_gpumem_bind_ranges {
            ranges: &range as *const _ as u64,
            ranges_nents: 1,
            ranges_size: std::mem::size_of::<kgsl_gpumem_bind_range>() as u32,
            id: self.vbo_id,
            flags: 0,
            fence_id: 0,
            padding: 0,
        };

        // SAFETY: Underlying descriptor is valid and req references valid range struct.
        unsafe {
            kgsl_ioctl_gpumem_bind_ranges(self.physical_device.as_fd().unwrap(), &mut req)?;
        }

        Ok(())
    }

    fn unmap_buffer_gpu(&self, gpu_va: u64, size: u64) -> MagmaGpuResult<()> {
        if self.vbo_id == 0 {
            return Ok(());
        }

        let target_offset = self.compute_target_offset(gpu_va);

        let range = kgsl_gpumem_bind_range {
            child_offset: 0,
            target_offset,
            length: size,
            child_id: 0,
            op: KGSL_GPUMEM_RANGE_OP_UNBIND,
        };

        let mut req = kgsl_gpumem_bind_ranges {
            ranges: &range as *const _ as u64,
            ranges_nents: 1,
            ranges_size: std::mem::size_of::<kgsl_gpumem_bind_range>() as u32,
            id: self.vbo_id,
            flags: 0,
            fence_id: 0,
            padding: 0,
        };

        // SAFETY: Underlying descriptor is valid and req references valid range struct.
        unsafe {
            kgsl_ioctl_gpumem_bind_ranges(self.physical_device.as_fd().unwrap(), &mut req)?;
        }

        Ok(())
    }
}

impl AddressSpace for KgslAddressSpace {}

pub struct KgslQueue {
    physical_device: Arc<dyn PhysicalDevice>,
    context_id: u32,
}

impl Drop for KgslQueue {
    fn drop(&mut self) {
        let destroy_req = kgsl_drawctxt_destroy {
            drawctxt_id: self.context_id,
        };
        // SAFETY: Descriptor is valid and context_id corresponds to a created drawctxt.
        unsafe {
            let _ =
                kgsl_ioctl_drawctxt_destroy(self.physical_device.as_fd().unwrap(), &destroy_req);
        }
    }
}

impl GenericQueue for KgslQueue {
    fn submit_command(&self, submit_info: &MagmaSubmitInfo) -> MagmaGpuResult<()> {
        let (cmd_id, cmd_gpuaddr, cmd_offset, cmd_size) =
            if let Some(as_info) = submit_info.address_space_info() {
                (0, as_info.command_va, 0, as_info.length)
            } else if let Some(buf_info) = submit_info.buffer_info() {
                (
                    buf_info.command_buffer,
                    0,
                    buf_info.start_offset,
                    buf_info.length,
                )
            } else {
                return Err(MagmaGpuError::Unsupported);
            };

        let cmd_obj = kgsl_command_object {
            offset: cmd_offset,
            gpuaddr: cmd_gpuaddr,
            size: cmd_size,
            flags: KGSL_CMDLIST_IB,
            id: cmd_id,
        };

        let mut sync_fences: Vec<kgsl_cmd_syncpoint_fence> = Vec::new();
        let mut syncpoints: Vec<kgsl_command_syncpoint> = Vec::new();

        if let Some(sync_info) = submit_info.sync_info() {
            for &handle in sync_info
                .wait_sync_objs
                .iter()
                .take(sync_info.num_wait_sync_objs as usize)
            {
                if handle != 0 {
                    sync_fences.push(kgsl_cmd_syncpoint_fence { fd: handle as i32 });
                }
            }
        }

        for fence in &sync_fences {
            syncpoints.push(kgsl_command_syncpoint {
                priv_: fence as *const _ as u64,
                size: std::mem::size_of::<kgsl_cmd_syncpoint_fence>() as u64,
                type_: KGSL_CMD_SYNCPOINT_TYPE_FENCE,
            });
        }

        let mut req = kgsl_gpu_command {
            flags: KGSL_CMDBATCH_SUBMIT_IB_LIST as u64,
            cmdlist: &cmd_obj as *const _ as u64,
            cmdsize: std::mem::size_of::<kgsl_command_object>() as u32,
            numcmds: 1,
            synclist: if syncpoints.is_empty() {
                0
            } else {
                syncpoints.as_ptr() as u64
            },
            syncsize: std::mem::size_of::<kgsl_command_syncpoint>() as u32,
            numsyncs: syncpoints.len() as u32,
            context_id: self.context_id,
            ..Default::default()
        };

        // SAFETY: Descriptor is valid and req references valid command object.
        unsafe {
            kgsl_ioctl_gpu_command(self.physical_device.as_fd().unwrap(), &mut req)?;
        }
        Ok(())
    }
}

impl Queue for KgslQueue {}

unsafe impl Send for Kgsl {}
unsafe impl Sync for Kgsl {}

unsafe impl Send for KgslAddressSpace {}
unsafe impl Sync for KgslAddressSpace {}

unsafe impl Send for KgslQueue {}
unsafe impl Sync for KgslQueue {}

unsafe impl Send for KgslBuffer {}
unsafe impl Sync for KgslBuffer {}
