// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use magma_gpu::util::Error as MagmaGpuError;
use magma_gpu::util::FromRawDescriptor;
use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MappedRegion;
use magma_gpu::util::OwnedDescriptor;
use magma_gpu::util::RawMapping;
use magma_gpu::util::Result as MagmaGpuResult;
use magma_gpu::util::DEFAULT_RAW_DESCRIPTOR;
use magma_gpu::util::MAGMA_GPU_HANDLE_TYPE_SIGNAL_SYNC_FD;
use magma_gpu::virtgpu_kumquat::defines::VirtGpuParam;
use magma_gpu::virtgpu_kumquat::defines::VirtGpuResourceCreateBlob;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_BLOB_FLAG_USE_MAPPABLE;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_BLOB_FLAG_USE_SHAREABLE;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_BLOB_MEM_HOST3D;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_EXECBUF_FENCE_FD_OUT;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_EXECBUF_RING_IDX;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_EXECBUF_SHAREABLE_OUT;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_PARAM_3D_FEATURES;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_PARAM_CAPSET_QUERY_FIX;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_PARAM_CONTEXT_INIT;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_PARAM_CROSS_DEVICE;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_PARAM_EXPLICIT_DEBUG_NAME;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_PARAM_FENCE_PASSING;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_PARAM_HOST_VISIBLE;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_PARAM_RESOURCE_BLOB;
use magma_gpu::virtgpu_kumquat::defines::VIRTGPU_KUMQUAT_PARAM_SUPPORTED_CAPSET_IDS;
use magma_gpu::virtgpu_kumquat::VirtGpuKumquat;

use zerocopy::TryFromBytes;

use crate::magma::MagmaPhysicalDevice;
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
use crate::protocol::MagmaGpuMapFlags;
use crate::sys::platform::PlatformDevice;
use crate::sys::platform::PlatformPhysicalDevice;
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

use crate::encoder::Encoder;
use crate::protocol::*;
use crate::ring::MagmaRingBuffer;
use crate::ring::RING_STATUS_ASLEEP;
use crate::virtgpu_shared::*;

/* should be in virtgpu_kumquat */
pub const VIRTGPU_KUMQUAT_CAPSET_MAGMA: u32 = 7;

fn query_param(virtgpu: &mut VirtGpuKumquat, param: u64) -> MagmaGpuResult<u64> {
    let mut p = VirtGpuParam { param, value: 0 };
    virtgpu.get_param(&mut p)?;
    Ok(p.value)
}

pub struct MagmaKumquat {
    virtgpu: Mutex<VirtGpuKumquat>,
    caps: MagmaVirtCapabilities,
    object_id: AtomicU32,
    cpu_ring: MagmaRingBuffer,
    _cpu_bo_handle: u32,
}

impl MagmaKumquat {
    pub fn new() -> MagmaGpuResult<MagmaKumquat> {
        let mut virtgpu_kumquat = VirtGpuKumquat::new("/tmp/kumquat-gpu-0")?;

        let _features_3d =
            query_param(&mut virtgpu_kumquat, VIRTGPU_KUMQUAT_PARAM_3D_FEATURES).unwrap_or(0);
        let _capset_query_fix =
            query_param(&mut virtgpu_kumquat, VIRTGPU_KUMQUAT_PARAM_CAPSET_QUERY_FIX).unwrap_or(0);
        let resource_blob =
            query_param(&mut virtgpu_kumquat, VIRTGPU_KUMQUAT_PARAM_RESOURCE_BLOB).unwrap_or(0);
        let _host_visible =
            query_param(&mut virtgpu_kumquat, VIRTGPU_KUMQUAT_PARAM_HOST_VISIBLE).unwrap_or(0);
        let _cross_device =
            query_param(&mut virtgpu_kumquat, VIRTGPU_KUMQUAT_PARAM_CROSS_DEVICE).unwrap_or(0);
        let context_init =
            query_param(&mut virtgpu_kumquat, VIRTGPU_KUMQUAT_PARAM_CONTEXT_INIT).unwrap_or(0);
        let capsets = query_param(
            &mut virtgpu_kumquat,
            VIRTGPU_KUMQUAT_PARAM_SUPPORTED_CAPSET_IDS,
        )
        .unwrap_or(0);
        let _debug_name = query_param(
            &mut virtgpu_kumquat,
            VIRTGPU_KUMQUAT_PARAM_EXPLICIT_DEBUG_NAME,
        )
        .unwrap_or(0);
        let _fence_passing =
            query_param(&mut virtgpu_kumquat, VIRTGPU_KUMQUAT_PARAM_FENCE_PASSING).unwrap_or(0);

        if resource_blob == 0
            || context_init == 0
            || (capsets & (1 << VIRTGPU_KUMQUAT_CAPSET_MAGMA)) == 0
        {
            return Err(MagmaGpuError::Unsupported);
        }

        let mut caps_bytes = [0u8; std::mem::size_of::<MagmaVirtCapabilities>()];
        virtgpu_kumquat.get_caps(VIRTGPU_KUMQUAT_CAPSET_MAGMA, &mut caps_bytes)?;
        let caps: MagmaVirtCapabilities = TryFromBytes::try_read_from_bytes(&caps_bytes)
            .map_err(|_| MagmaGpuError::Unsupported)?;

        let _ctx_id =
            virtgpu_kumquat.context_create(VIRTGPU_KUMQUAT_CAPSET_MAGMA as u64, "magma")?;

        // TODO: Add ring_size to the virtualization capabilities.
        const RING_SIZE: usize = 65536;
        let mut create_blob = VirtGpuResourceCreateBlob {
            // TODO: Fix kumquat to support VIRTGPU_BLOB_MEM_GUEST.
            blob_mem: VIRTGPU_BLOB_MEM_HOST3D,
            blob_flags: VIRTGPU_BLOB_FLAG_USE_MAPPABLE | VIRTGPU_BLOB_FLAG_USE_SHAREABLE,
            bo_handle: 0,
            res_handle: 0,
            size: RING_SIZE as u64,
            pad: 0,
            cmd_size: 0,
            cmd: 0,
            blob_id: 1,
        };

        virtgpu_kumquat.resource_create_blob(&mut create_blob, &[])?;
        let raw_mapping = virtgpu_kumquat.map(create_blob.bo_handle)?;

        let mut raw_desc = DEFAULT_RAW_DESCRIPTOR;
        encode_and_submit_virt_create_render_thread(0, create_blob.res_handle, |cmd| {
            virtgpu_kumquat.submit_command(0, &[], cmd, 0, &[], &mut raw_desc)
        })?;

        let cpu_ring = unsafe {
            MagmaRingBuffer::from_raw_parts(raw_mapping.ptr as *mut u8, raw_mapping.size as usize)?
        };

        Ok(MagmaKumquat {
            virtgpu: Mutex::new(virtgpu_kumquat),
            caps,
            object_id: AtomicU32::new(1),
            cpu_ring,
            _cpu_bo_handle: create_blob.bo_handle,
        })
    }

    fn submit_encoded_cmd(&self, cmd: &[u8]) -> MagmaGpuResult<()> {
        let mut raw_desc = DEFAULT_RAW_DESCRIPTOR;
        let mut virtgpu = self
            .virtgpu
            .lock()
            .map_err(|_| MagmaGpuError::WithContext("lock failed"))?;
        virtgpu.submit_command(0, &[], cmd, 0, &[], &mut raw_desc)
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

impl AsVirtGpu for MagmaKumquat {
    fn as_virtgpu(&self) -> Option<&Mutex<VirtGpuKumquat>> {
        Some(&self.virtgpu)
    }
}

impl PlatformPhysicalDevice for MagmaKumquat {}
impl PhysicalDevice for MagmaKumquat {}

impl PlatformDevice for MagmaKumquat {}
impl Device for MagmaKumquat {}

impl GenericPhysicalDevice for MagmaKumquat {
    fn create_device(
        self: Arc<Self>,
        _info: &MagmaPhysicalDeviceInfo,
    ) -> MagmaGpuResult<Arc<dyn Device>> {
        encode_and_submit_create_device(|cmd| self.submit_encoded_cpu_cmd(cmd))?;
        Ok(self.clone())
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

impl GenericDevice for MagmaKumquat {
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
        Ok(Arc::new(MagmaKumquatAddressSpace {
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

        // TODO: Add ring_size to the virtualization capabilities.
        const RING_SIZE: usize = 65536;
        let mut create_blob = VirtGpuResourceCreateBlob {
            // TODO: Fix kumquat to support VIRTGPU_BLOB_MEM_GUEST.
            blob_mem: VIRTGPU_BLOB_MEM_HOST3D,
            blob_flags: VIRTGPU_BLOB_FLAG_USE_MAPPABLE | VIRTGPU_BLOB_FLAG_USE_SHAREABLE,
            bo_handle: 0,
            res_handle: 0,
            size: RING_SIZE as u64,
            pad: 0,
            cmd_size: 0,
            cmd: 0,
            blob_id: 1,
        };

        let raw_mapping = {
            let mut virtgpu = self
                .virtgpu
                .lock()
                .map_err(|_| MagmaGpuError::WithContext("lock failed"))?;
            virtgpu.resource_create_blob(&mut create_blob, &[])?;
            virtgpu.map(create_blob.bo_handle)?
        };

        encode_and_submit_virt_create_render_thread(object_id, create_blob.res_handle, |cmd| {
            self.submit_encoded_cmd(cmd)
        })?;

        let ring = unsafe {
            MagmaRingBuffer::from_raw_parts(raw_mapping.ptr as *mut u8, raw_mapping.size as usize)?
        };

        Ok(Arc::new(MagmaKumquatQueue {
            parent: self.clone(),
            object_id,
            ring,
            _bo_handle: create_blob.bo_handle,
        }))
    }

    fn create_buffer(
        self: Arc<Self>,
        create_info: &MagmaCreateBufferInfo,
    ) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let mut create_blob = VirtGpuResourceCreateBlob {
            // TODO: Fix kumquat to support VIRTGPU_BLOB_MEM_GUEST.
            blob_mem: VIRTGPU_BLOB_MEM_HOST3D,
            blob_flags: VIRTGPU_BLOB_FLAG_USE_MAPPABLE | VIRTGPU_BLOB_FLAG_USE_SHAREABLE,
            bo_handle: 0,
            res_handle: 0,
            size: create_info.size,
            pad: 0,
            cmd_size: 0,
            cmd: 0,
            blob_id: 1,
        };

        {
            let mut virtgpu = self
                .virtgpu
                .lock()
                .map_err(|_| MagmaGpuError::WithContext("lock failed"))?;
            virtgpu.resource_create_blob(&mut create_blob, &[])?;
        }
        let object_id = self.object_id.fetch_add(1, Ordering::Relaxed);

        encode_and_submit_create_buffer(create_info, |cmd| self.submit_encoded_cpu_cmd(cmd))?;

        Ok(Arc::new(MagmaKumquatBuffer {
            parent: self.clone(),
            object_id,
            bo_handle: create_blob.bo_handle,
            _res_handle: create_blob.res_handle,
            _size: create_info.size as usize,
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
            // TODO: Formalize seqno in Gorgonzola instead of using _pad0.
            _pad0: seqno,
            info: *info,
        };
        encoder.encode_create_sync_obj(&req)?;
        let len = encoder.bytes_written();
        self.submit_encoded_cpu_cmd(&buf[..len])?;
        Ok(Arc::new(MagmaKumquatSyncObject {
            parent: self.clone(),
            object_id,
            ring_idx: object_id,
        }))
    }
}

pub struct MagmaKumquatSyncObject {
    parent: Arc<MagmaKumquat>,
    object_id: u32,
    ring_idx: u32,
}

impl GenericSyncObject for MagmaKumquatSyncObject {
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
        let mut virtgpu = self
            .parent
            .virtgpu
            .lock()
            .map_err(|_| MagmaGpuError::WithContext("lock failed"))?;
        let mut raw_desc = DEFAULT_RAW_DESCRIPTOR;
        virtgpu.submit_command(
            VIRTGPU_KUMQUAT_EXECBUF_RING_IDX
                | VIRTGPU_KUMQUAT_EXECBUF_SHAREABLE_OUT
                | VIRTGPU_KUMQUAT_EXECBUF_FENCE_FD_OUT,
            &[],
            &[],
            self.ring_idx,
            &[],
            &mut raw_desc,
        )?;
        let descriptor = unsafe { OwnedDescriptor::from_raw_descriptor(raw_desc) };
        Ok(MagmaGpuHandle {
            os_handle: descriptor,
            handle_type: MAGMA_GPU_HANDLE_TYPE_SIGNAL_SYNC_FD,
        })
    }

    fn as_raw_handle(&self) -> Option<u32> {
        Some(self.object_id)
    }
}

impl SyncObject for MagmaKumquatSyncObject {}

pub struct MagmaKumquatAddressSpace {
    parent: Arc<MagmaKumquat>,
    object_id: u32,
}

impl GenericAddressSpace for MagmaKumquatAddressSpace {
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

impl AddressSpace for MagmaKumquatAddressSpace {}

pub struct MagmaKumquatQueue {
    parent: Arc<MagmaKumquat>,
    object_id: u32,
    ring: MagmaRingBuffer,
    _bo_handle: u32,
}

impl GenericQueue for MagmaKumquatQueue {
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
            // TODO: Formalize wait_seqno in Gorgonzola instead of using _pad0.
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

impl Queue for MagmaKumquatQueue {}

pub struct MagmaKumquatBuffer {
    parent: Arc<MagmaKumquat>,
    object_id: u32,
    bo_handle: u32,
    _res_handle: u32,
    _size: usize,
}

struct KumquatMappedRegion(RawMapping);

unsafe impl Send for KumquatMappedRegion {}
unsafe impl Sync for KumquatMappedRegion {}

unsafe impl MappedRegion for KumquatMappedRegion {
    fn as_ptr(&self) -> *mut u8 {
        self.0.ptr as *mut u8
    }

    fn size(&self) -> usize {
        self.0.size as usize
    }

    fn as_raw_mapping(&self) -> RawMapping {
        self.0
    }
}

impl GenericBuffer for MagmaKumquatBuffer {
    fn map(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn MappedRegion>> {
        let mut virtgpu = self
            .parent
            .virtgpu
            .lock()
            .map_err(|_| MagmaGpuError::WithContext("lock failed"))?;
        let raw_mapping = virtgpu.map(self.bo_handle)?;
        Ok(Arc::new(KumquatMappedRegion(raw_mapping)))
    }

    fn export(&self) -> MagmaGpuResult<MagmaGpuHandle> {
        Err(MagmaGpuError::Unsupported)
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

impl Buffer for MagmaKumquatBuffer {}

pub fn enumerate_devices() -> MagmaGpuResult<Vec<MagmaPhysicalDevice>> {
    let enc = Arc::new(MagmaKumquat::new()?);
    let mut devices: Vec<MagmaPhysicalDevice> = Vec::new();

    for i in 0..enc.caps.num_physical_devices as usize {
        if i >= enc.caps.devices.len() {
            break;
        }
        let info = enc.caps.devices[i];
        devices.push(MagmaPhysicalDevice::new(enc.clone(), info));
    }

    Ok(devices)
}
