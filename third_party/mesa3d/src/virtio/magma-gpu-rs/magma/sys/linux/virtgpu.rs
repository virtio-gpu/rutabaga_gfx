// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

#![allow(dead_code)]

use std::os::fd::AsFd;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::ioctl_readwrite;

use magma_gpu::util::Error as MagmaGpuError;
use magma_gpu::util::FromRawDescriptor;
use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MappedRegion;
use magma_gpu::util::OwnedDescriptor;
use magma_gpu::util::Result as MagmaGpuResult;
use magma_gpu::util::MAGMA_GPU_HANDLE_TYPE_SIGNAL_SYNC_FD;

use zerocopy::TryFromBytes;

use crate::encoder::Encoder;
use crate::protocol::*;
use crate::ring::MagmaRingBuffer;
use crate::ring::RING_STATUS_ASLEEP;
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
use crate::magma_defines::MagmaHeapBudget;
use crate::magma_defines::MagmaImportHandleInfo;
use crate::magma_defines::MagmaMappedMemoryRange;
use crate::magma_defines::MagmaMemoryProperties;
use crate::magma_defines::MagmaPhysicalDeviceInfo;
use crate::magma_defines::MagmaQueueFamilyProperties;
use crate::magma_defines::MagmaResult;
use crate::magma_defines::MagmaSubmitInfo;

use crate::sys::linux::bindings::drm_bindings::DRM_COMMAND_BASE;
use crate::sys::linux::bindings::drm_bindings::DRM_IOCTL_BASE;
use crate::sys::linux::bindings::virtgpu_bindings::*;
use crate::sys::linux::PlatformDevice;
use crate::sys::linux::PlatformPhysicalDevice;
use crate::virtgpu_shared::*;

pub const VIRTGPU_CAPSET_MAGMA: u32 = 7;

ioctl_readwrite!(
    drm_ioctl_virtgpu_map,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_VIRTGPU_MAP,
    drm_virtgpu_map
);

ioctl_readwrite!(
    drm_ioctl_virtgpu_execbuffer,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_VIRTGPU_EXECBUFFER,
    drm_virtgpu_execbuffer
);

ioctl_readwrite!(
    drm_ioctl_virtgpu_getparam,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_VIRTGPU_GETPARAM,
    drm_virtgpu_getparam
);

ioctl_readwrite!(
    drm_ioctl_virtgpu_get_caps,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_VIRTGPU_GET_CAPS,
    drm_virtgpu_get_caps
);

ioctl_readwrite!(
    drm_ioctl_virtgpu_resource_create_blob,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_VIRTGPU_RESOURCE_CREATE_BLOB,
    drm_virtgpu_resource_create_blob
);

ioctl_readwrite!(
    drm_ioctl_virtgpu_context_init,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_VIRTGPU_CONTEXT_INIT,
    drm_virtgpu_context_init
);

fn query_param<F: AsFd>(fd: &F, param: u64) -> MagmaGpuResult<u64> {
    let mut getparam = drm_virtgpu_getparam { param, value: 0 };
    unsafe {
        drm_ioctl_virtgpu_getparam(fd.as_fd(), &mut getparam)?;
    }
    Ok(getparam.value)
}

#[derive(Debug)]
pub struct VirtGpuPhysicalDevice {
    descriptor: OwnedDescriptor,
    caps: MagmaVirtCapabilities,
}

impl VirtGpuPhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> MagmaGpuResult<Self> {
        let _features_3d = query_param(&descriptor, VIRTGPU_PARAM_3D_FEATURES as u64).unwrap_or(0);
        let _capset_query_fix =
            query_param(&descriptor, VIRTGPU_PARAM_CAPSET_QUERY_FIX as u64).unwrap_or(0);
        let resource_blob =
            query_param(&descriptor, VIRTGPU_PARAM_RESOURCE_BLOB as u64).unwrap_or(0);
        let _host_visible =
            query_param(&descriptor, VIRTGPU_PARAM_HOST_VISIBLE as u64).unwrap_or(0);
        let _cross_device =
            query_param(&descriptor, VIRTGPU_PARAM_CROSS_DEVICE as u64).unwrap_or(0);
        let context_init = query_param(&descriptor, VIRTGPU_PARAM_CONTEXT_INIT as u64).unwrap_or(0);
        let capsets =
            query_param(&descriptor, VIRTGPU_PARAM_SUPPORTED_CAPSET_IDs as u64).unwrap_or(0);
        let _debug_name =
            query_param(&descriptor, VIRTGPU_PARAM_EXPLICIT_DEBUG_NAME as u64).unwrap_or(0);

        if resource_blob == 0 || context_init == 0 || (capsets & (1 << VIRTGPU_CAPSET_MAGMA)) == 0 {
            return Err(MagmaGpuError::Unsupported);
        }

        let mut caps_bytes = [0u8; std::mem::size_of::<MagmaVirtCapabilities>()];
        let mut get_caps = drm_virtgpu_get_caps {
            cap_set_id: VIRTGPU_CAPSET_MAGMA,
            cap_set_ver: 0,
            addr: caps_bytes.as_mut_ptr() as u64,
            size: std::mem::size_of::<MagmaVirtCapabilities>() as u32,
            pad: 0,
        };
        unsafe {
            drm_ioctl_virtgpu_get_caps(descriptor.as_fd(), &mut get_caps)?;
        }
        let caps: MagmaVirtCapabilities = TryFromBytes::try_read_from_bytes(&caps_bytes)
            .map_err(|_| MagmaGpuError::Unsupported)?;

        Ok(Self { descriptor, caps })
    }

    fn submit_encoded_cmd(&self, cmd: &[u8]) -> MagmaGpuResult<()> {
        let mut execbuf = drm_virtgpu_execbuffer {
            flags: 0,
            size: cmd.len() as u32,
            command: cmd.as_ptr() as u64,
            ..Default::default()
        };
        unsafe {
            drm_ioctl_virtgpu_execbuffer(self.descriptor.as_fd(), &mut execbuf)?;
        }
        Ok(())
    }
}

impl PlatformPhysicalDevice for VirtGpuPhysicalDevice {
    fn as_descriptor(&self) -> Option<&OwnedDescriptor> {
        Some(&self.descriptor)
    }
}

impl AsVirtGpu for VirtGpuPhysicalDevice {}
impl PhysicalDevice for VirtGpuPhysicalDevice {}
unsafe impl Send for VirtGpuPhysicalDevice {}
unsafe impl Sync for VirtGpuPhysicalDevice {}

impl GenericPhysicalDevice for VirtGpuPhysicalDevice {
    fn create_device(
        self: Arc<Self>,
        _info: &MagmaPhysicalDeviceInfo,
    ) -> MagmaGpuResult<Arc<dyn Device>> {
        Ok(Arc::new(VirtGpu::new(self)?))
    }

    fn query_memory_properties(&self) -> MagmaGpuResult<MagmaMemoryProperties> {
        Ok(self.caps.devices[0].memory_properties)
    }

    fn query_queue_family_properties(&self) -> MagmaGpuResult<Vec<MagmaQueueFamilyProperties>> {
        let count = (self.caps.devices[0].queue_family_count as usize)
            .min(self.caps.devices[0].queue_families.len());
        Ok(self.caps.devices[0].queue_families[..count].to_vec())
    }
}

pub struct VirtGpu {
    physical_device: Arc<VirtGpuPhysicalDevice>,
    caps: MagmaVirtCapabilities,
    object_id: AtomicU32,
    cpu_ring: MagmaRingBuffer,
    _cpu_bo_handle: u32,
    _cpu_mapping: Arc<dyn MappedRegion>,
}

impl VirtGpu {
    pub fn new(physical_device: Arc<VirtGpuPhysicalDevice>) -> MagmaGpuResult<VirtGpu> {
        let ctx_params = [drm_virtgpu_context_set_param {
            param: VIRTGPU_CONTEXT_PARAM_CAPSET_ID as u64,
            value: VIRTGPU_CAPSET_MAGMA as u64,
        }];
        let mut init = drm_virtgpu_context_init {
            num_params: ctx_params.len() as u32,
            pad: 0,
            ctx_set_params: ctx_params.as_ptr() as u64,
        };
        unsafe {
            drm_ioctl_virtgpu_context_init(physical_device.as_fd().unwrap(), &mut init)?;
        }

        const RING_SIZE: usize = 65536;
        let mut create_blob = drm_virtgpu_resource_create_blob {
            blob_mem: VIRTGPU_BLOB_MEM_HOST3D,
            blob_flags: VIRTGPU_BLOB_FLAG_USE_MAPPABLE | VIRTGPU_BLOB_FLAG_USE_SHAREABLE,
            size: RING_SIZE as u64,
            blob_id: 1,
            ..Default::default()
        };
        unsafe {
            drm_ioctl_virtgpu_resource_create_blob(
                physical_device.as_fd().unwrap(),
                &mut create_blob,
            )?;
        }

        let mut map_arg = drm_virtgpu_map {
            handle: create_blob.bo_handle,
            ..Default::default()
        };
        let offset = unsafe {
            drm_ioctl_virtgpu_map(physical_device.as_fd().unwrap(), &mut map_arg)?;
            map_arg.offset
        };
        let mapping = physical_device.cpu_map(offset, RING_SIZE)?;
        let cpu_ring = unsafe { MagmaRingBuffer::from_raw_parts(mapping.as_ptr(), RING_SIZE)? };

        let virtgpu = VirtGpu {
            physical_device: physical_device.clone(),
            caps: physical_device.caps,
            object_id: AtomicU32::new(1),
            cpu_ring,
            _cpu_bo_handle: create_blob.bo_handle,
            _cpu_mapping: Arc::new(mapping),
        };

        encode_and_submit_virt_create_render_thread(0, create_blob.res_handle, |cmd| {
            virtgpu.submit_encoded_cmd(cmd)
        })?;

        encode_and_submit_create_device(|cmd| virtgpu.submit_encoded_cpu_cmd(cmd))?;

        Ok(virtgpu)
    }

    fn submit_encoded_cmd(&self, cmd: &[u8]) -> MagmaGpuResult<()> {
        let mut execbuf = drm_virtgpu_execbuffer {
            flags: 0,
            size: cmd.len() as u32,
            command: cmd.as_ptr() as u64,
            ..Default::default()
        };
        unsafe {
            drm_ioctl_virtgpu_execbuffer(self.physical_device.as_fd().unwrap(), &mut execbuf)?;
        }
        Ok(())
    }

    fn submit_encoded_cpu_cmd(&self, cmd: &[u8]) -> MagmaGpuResult<()> {
        let next_seq = self.cpu_ring.submitted_seqno().wrapping_add(1);
        self.cpu_ring.set_submitted_seqno(next_seq);
        self.cpu_ring.write(cmd)?;
        std::sync::atomic::fence(Ordering::SeqCst);

        if self.cpu_ring.status() == RING_STATUS_ASLEEP {
            encode_and_submit_virt_ping(0, |ping_cmd| self.submit_encoded_cmd(ping_cmd))?;
        }
        Ok(())
    }
}

impl GenericDevice for VirtGpu {
    fn get_memory_budget(&self, heap_idx: u32) -> MagmaGpuResult<MagmaHeapBudget> {
        let mem_props = &self.caps.devices[0].memory_properties;
        if (heap_idx as usize) < mem_props.memory_heap_count as usize {
            let heap = &mem_props.memory_heaps[heap_idx as usize];
            Ok(MagmaHeapBudget {
                budget: heap.heap_size,
                usage: 0,
            })
        } else {
            Err(MagmaGpuError::WithContext("Heap Index out of bounds"))
        }
    }

    fn create_address_space(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn AddressSpace>> {
        let object_id = self.object_id.fetch_add(1, Ordering::Relaxed);
        encode_and_submit_create_address_space(|cmd| self.submit_encoded_cpu_cmd(cmd))?;
        Ok(Arc::new(VirtGpuAddressSpace {
            parent: self.clone(),
            object_id,
        }))
    }

    fn create_queue(
        self: Arc<Self>,
        address_space: &Arc<dyn AddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> MagmaGpuResult<Arc<dyn Queue>> {
        let object_id = self.object_id.fetch_add(1, Ordering::Relaxed);
        let as_id = address_space.as_vm_id().unwrap_or(1);
        encode_and_submit_create_queue(as_id, info, |cmd| self.submit_encoded_cpu_cmd(cmd))?;

        const RING_SIZE: usize = 65536;
        let mut create_blob = drm_virtgpu_resource_create_blob {
            blob_mem: VIRTGPU_BLOB_MEM_HOST3D,
            blob_flags: VIRTGPU_BLOB_FLAG_USE_MAPPABLE | VIRTGPU_BLOB_FLAG_USE_SHAREABLE,
            size: RING_SIZE as u64,
            blob_id: 1,
            ..Default::default()
        };

        unsafe {
            drm_ioctl_virtgpu_resource_create_blob(
                self.physical_device.as_fd().unwrap(),
                &mut create_blob,
            )?;
        }

        let mut map_arg = drm_virtgpu_map {
            handle: create_blob.bo_handle,
            ..Default::default()
        };
        let offset = unsafe {
            drm_ioctl_virtgpu_map(self.physical_device.as_fd().unwrap(), &mut map_arg)?;
            map_arg.offset
        };
        let mapping = self.physical_device.cpu_map(offset, RING_SIZE)?;
        let ring = unsafe { MagmaRingBuffer::from_raw_parts(mapping.as_ptr(), RING_SIZE)? };

        encode_and_submit_virt_create_render_thread(object_id, create_blob.res_handle, |cmd| {
            self.submit_encoded_cmd(cmd)
        })?;

        Ok(Arc::new(VirtGpuQueue {
            parent: self.clone(),
            object_id,
            ring,
            _bo_handle: create_blob.bo_handle,
            _mapping: Arc::new(mapping),
        }))
    }

    fn create_buffer(
        self: Arc<Self>,
        create_info: &MagmaCreateBufferInfo,
    ) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let mut create_blob = drm_virtgpu_resource_create_blob {
            blob_mem: VIRTGPU_BLOB_MEM_HOST3D,
            blob_flags: VIRTGPU_BLOB_FLAG_USE_MAPPABLE | VIRTGPU_BLOB_FLAG_USE_SHAREABLE,
            size: create_info.size,
            blob_id: 1,
            ..Default::default()
        };

        unsafe {
            drm_ioctl_virtgpu_resource_create_blob(
                self.physical_device.as_fd().unwrap(),
                &mut create_blob,
            )?;
        }

        let object_id = self.object_id.fetch_add(1, Ordering::Relaxed);
        encode_and_submit_create_buffer(create_info, |cmd| self.submit_encoded_cpu_cmd(cmd))?;

        Ok(Arc::new(VirtGpuBuffer {
            parent: self.clone(),
            object_id,
            gem_handle: create_blob.bo_handle,
            _res_handle: create_blob.res_handle,
            size: create_info.size as usize,
        }))
    }

    fn import(self: Arc<Self>, _info: MagmaImportHandleInfo) -> MagmaGpuResult<Arc<dyn Buffer>> {
        Err(MagmaGpuError::Unsupported)
    }

    fn create_sync_obj(
        self: Arc<Self>,
        info: &MagmaCreateSyncObjInfo,
    ) -> MagmaGpuResult<Arc<dyn SyncObject>> {
        let object_id = self.object_id.fetch_add(1, Ordering::Relaxed);
        let mut buf = [0u8; 256];
        let mut encoder = Encoder::new(&mut buf);
        let seqno = self.cpu_ring.submitted_seqno().wrapping_add(1);
        let req = CreateSyncObj {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_CREATE_SYNC_OBJ,
                size: std::mem::size_of::<CreateSyncObj>() as u32,
            },
            device: 0,
            _pad0: seqno,
            info: *info,
        };
        encoder.encode_create_sync_obj(&req)?;
        let len = encoder.bytes_written();
        self.submit_encoded_cpu_cmd(&buf[..len])?;
        Ok(Arc::new(VirtGpuSyncObject {
            parent: self.clone(),
            object_id,
            ring_idx: object_id,
        }))
    }
}

pub struct VirtGpuSyncObject {
    parent: Arc<VirtGpu>,
    object_id: u32,
    ring_idx: u32,
}

impl GenericSyncObject for VirtGpuSyncObject {
    fn wait(&self, timeout_ns: u64) -> MagmaResult<()> {
        encode_and_submit_sync_obj_wait(self.object_id, timeout_ns, |cmd| {
            self.parent.submit_encoded_cpu_cmd(cmd)
        })
        .map_err(Into::into)
    }

    fn signal(&self) -> MagmaGpuResult<()> {
        encode_and_submit_sync_obj_signal(self.object_id, |cmd| {
            self.parent.submit_encoded_cpu_cmd(cmd)
        })
    }

    fn export_fence(&self) -> MagmaGpuResult<MagmaGpuHandle> {
        encode_and_submit_virt_create_fence(self.ring_idx, self.object_id, |cmd| {
            self.parent.submit_encoded_cpu_cmd(cmd)
        })?;
        let mut execbuf = drm_virtgpu_execbuffer {
            flags: VIRTGPU_EXECBUF_RING_IDX | VIRTGPU_EXECBUF_FENCE_FD_OUT,
            size: 0,
            command: 0,
            ring_idx: self.ring_idx,
            fence_fd: -1,
            ..Default::default()
        };
        unsafe {
            drm_ioctl_virtgpu_execbuffer(
                self.parent.physical_device.as_fd().unwrap(),
                &mut execbuf,
            )?;
        }
        let descriptor = unsafe { OwnedDescriptor::from_raw_descriptor(execbuf.fence_fd) };
        Ok(MagmaGpuHandle {
            os_handle: descriptor,
            handle_type: MAGMA_GPU_HANDLE_TYPE_SIGNAL_SYNC_FD,
        })
    }

    fn as_raw_handle(&self) -> Option<u32> {
        Some(self.object_id)
    }
}

impl SyncObject for VirtGpuSyncObject {}

pub struct VirtGpuAddressSpace {
    parent: Arc<VirtGpu>,
    object_id: u32,
}

impl GenericAddressSpace for VirtGpuAddressSpace {
    fn as_vm_id(&self) -> Option<u32> {
        Some(self.object_id)
    }

    fn map_buffer_gpu(
        &self,
        buffer: &Arc<dyn Buffer>,
        buffer_offset: u64,
        gpu_va: u64,
        size: u64,
        flags: MagmaGpuMapFlags,
    ) -> MagmaGpuResult<()> {
        let buf_id = buffer.as_gem_handle().unwrap_or(1);
        encode_and_submit_map_buffer_gpu(
            self.object_id,
            buf_id,
            buffer_offset,
            gpu_va,
            size,
            flags,
            |cmd| self.parent.submit_encoded_cpu_cmd(cmd),
        )
    }

    fn unmap_buffer_gpu(&self, gpu_va: u64, size: u64) -> MagmaGpuResult<()> {
        encode_and_submit_unmap_buffer_gpu(self.object_id, gpu_va, size, |cmd| {
            self.parent.submit_encoded_cpu_cmd(cmd)
        })
    }
}

impl AddressSpace for VirtGpuAddressSpace {}

pub struct VirtGpuQueue {
    parent: Arc<VirtGpu>,
    object_id: u32,
    ring: MagmaRingBuffer,
    _bo_handle: u32,
    _mapping: Arc<dyn MappedRegion>,
}

impl GenericQueue for VirtGpuQueue {
    fn submit_command(&self, submit_info: &MagmaSubmitInfo) -> MagmaGpuResult<()> {
        let wait_seq = self.parent.cpu_ring.submitted_seqno();
        let mut buf = [0u8; 512];
        let mut encoder = Encoder::new(&mut buf);
        let req = SubmitCommand {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_SUBMIT_COMMAND,
                size: std::mem::size_of::<SubmitCommand>() as u32,
            },
            queue: self.object_id,
            _pad0: wait_seq,
            submit_info: *submit_info,
        };
        encoder.encode_submit_command(&req)?;
        let len = encoder.bytes_written();
        let encoded_cmd = &buf[..len];

        self.ring.write(encoded_cmd)?;
        std::sync::atomic::fence(Ordering::SeqCst);

        if self.ring.status() == RING_STATUS_ASLEEP {
            encode_and_submit_virt_ping(self.object_id, |cmd| self.parent.submit_encoded_cmd(cmd))?;
        }

        Ok(())
    }
}

impl Queue for VirtGpuQueue {}

pub struct VirtGpuBuffer {
    parent: Arc<VirtGpu>,
    object_id: u32,
    gem_handle: u32,
    _res_handle: u32,
    size: usize,
}

impl GenericBuffer for VirtGpuBuffer {
    fn map(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn MappedRegion>> {
        let mut map_arg = drm_virtgpu_map {
            handle: self.gem_handle,
            ..Default::default()
        };
        let offset = unsafe {
            drm_ioctl_virtgpu_map(self.parent.physical_device.as_fd().unwrap(), &mut map_arg)?;
            map_arg.offset
        };
        let mapping = self.parent.physical_device.cpu_map(offset, self.size)?;
        Ok(Arc::new(mapping))
    }

    fn export(&self) -> MagmaGpuResult<MagmaGpuHandle> {
        self.parent.physical_device.export(self.gem_handle)
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
        Some(self.object_id)
    }
}

impl Buffer for VirtGpuBuffer {}

impl PlatformDevice for VirtGpu {}
impl Device for VirtGpu {}

unsafe impl Send for VirtGpu {}
unsafe impl Sync for VirtGpu {}
unsafe impl Send for VirtGpuAddressSpace {}
unsafe impl Sync for VirtGpuAddressSpace {}
unsafe impl Send for VirtGpuQueue {}
unsafe impl Sync for VirtGpuQueue {}
unsafe impl Send for VirtGpuBuffer {}
unsafe impl Sync for VirtGpuBuffer {}
unsafe impl Send for VirtGpuSyncObject {}
unsafe impl Sync for VirtGpuSyncObject {}
