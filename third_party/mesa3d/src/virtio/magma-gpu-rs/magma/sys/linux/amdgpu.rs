// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::os::fd::AsFd;
use std::os::fd::BorrowedFd;
use std::sync::Arc;

use log::error;
use magma_gpu::log_status;
use magma_gpu::util::Error as MagmaGpuError;
use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MappedRegion;
use magma_gpu::util::OwnedDescriptor;
use magma_gpu::util::Result as MagmaGpuResult;

use crate::ioctl_readwrite;
use crate::ioctl_write_ptr;

use crate::magma_defines::MagmaCreateBufferInfo;
use crate::magma_defines::MagmaCreateQueueInfo;
use crate::magma_defines::MagmaCreateSyncObjInfo;
use crate::magma_defines::MagmaHeapBudget;
use crate::magma_defines::MagmaImportHandleInfo;
use crate::magma_defines::MagmaMappedMemoryRange;
use crate::magma_defines::MagmaMemoryProperties;
use crate::magma_defines::MagmaQueueFamilyProperties;
use crate::magma_defines::MagmaQueueFlags;
use crate::magma_defines::MagmaSubmitInfo;
use crate::magma_defines::MagmaSyncType;
use crate::magma_defines::MAGMA_BUFFER_FLAG_AMD_GDS;
use crate::magma_defines::MAGMA_BUFFER_FLAG_AMD_OA;
use crate::magma_defines::MAGMA_HEAP_CPU_VISIBLE_BIT;
use crate::magma_defines::MAGMA_HEAP_DEVICE_LOCAL_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT;
use crate::protocol::MagmaGpuMapFlags;

use crate::magma_defines::MagmaPhysicalDeviceInfo;
use crate::sys::linux::bindings::amdgpu_bindings::*;
use crate::sys::linux::bindings::drm_bindings::DRM_COMMAND_BASE;
use crate::sys::linux::bindings::drm_bindings::DRM_IOCTL_BASE;
use crate::sys::linux::drm::DrmSyncObject;
use crate::sys::linux::PlatformDevice;
use crate::sys::linux::PlatformPhysicalDevice;

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

ioctl_readwrite!(
    drm_ioctl_amdgpu_cs,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_AMDGPU_CS,
    drm_amdgpu_cs
);

ioctl_readwrite!(
    drm_ioctl_amdgpu_ctx,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_AMDGPU_CTX,
    drm_amdgpu_ctx
);

ioctl_readwrite!(
    drm_ioctl_amdgpu_gem_va,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_AMDGPU_GEM_VA,
    drm_amdgpu_gem_va
);

ioctl_write_ptr!(
    drm_ioctl_amdgpu_info,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_AMDGPU_INFO,
    drm_amdgpu_info
);

macro_rules! amdgpu_info_ioctl {
    ($(#[$attr:meta])* $name:ident, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: BorrowedFd<'_>,
                            data: *mut $ty)
                            -> MagmaGpuResult<()> {
            let mut info: drm_amdgpu_info = Default::default();
            info.query = $nr;
            info.return_size = ::std::mem::size_of::<$ty>() as u32;
            info.return_pointer = data as __u64;
            drm_ioctl_amdgpu_info(fd, &info)?;
            Ok(())
        }
    )
}

amdgpu_info_ioctl!(
    drm_ioctl_amdgpu_info_memory,
    AMDGPU_INFO_MEMORY,
    drm_amdgpu_memory_info
);

amdgpu_info_ioctl!(
    drm_ioctl_amdgpu_info_vram_gtt,
    AMDGPU_INFO_VRAM_GTT,
    drm_amdgpu_info_vram_gtt
);

amdgpu_info_ioctl!(drm_ioctl_amdgpu_info_gtt_usage, AMDGPU_INFO_GTT_USAGE, u64);

amdgpu_info_ioctl!(
    drm_ioctl_amdgpu_info_vram_usage,
    AMDGPU_INFO_VRAM_USAGE,
    u64
);

amdgpu_info_ioctl!(
    drm_ioctl_amdgpu_info_vis_vram_usage,
    AMDGPU_INFO_VIS_VRAM_USAGE,
    u64
);

pub unsafe fn drm_ioctl_amdgpu_info_hw_ip(
    fd: BorrowedFd<'_>,
    type_: u32,
    ip_instance: u32,
    data: *mut drm_amdgpu_info_hw_ip,
) -> MagmaGpuResult<()> {
    let info = drm_amdgpu_info {
        return_pointer: data as __u64,
        return_size: ::std::mem::size_of::<drm_amdgpu_info_hw_ip>() as u32,
        query: AMDGPU_INFO_HW_IP_INFO,
        __bindgen_anon_1: drm_amdgpu_info__bindgen_ty_1 {
            query_hw_ip: drm_amdgpu_info__bindgen_ty_1__bindgen_ty_2 { type_, ip_instance },
        },
    };
    drm_ioctl_amdgpu_info(fd, &info)?;
    Ok(())
}

ioctl_readwrite!(
    drm_ioctl_amdgpu_gem_create,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_AMDGPU_GEM_CREATE,
    drm_amdgpu_gem_create
);

ioctl_readwrite!(
    drm_ioctl_amdgpu_gem_mmap,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_AMDGPU_GEM_MMAP,
    drm_amdgpu_gem_mmap
);

pub struct AmdGpu {
    physical_device: Arc<dyn PhysicalDevice>,
    mem_props: MagmaMemoryProperties,
}

struct AmdGpuBuffer {
    physical_device: Arc<dyn PhysicalDevice>,
    gem_handle: u32,
    size: usize,
}

#[derive(Debug)]
pub struct AmdGpuPhysicalDevice {
    descriptor: OwnedDescriptor,
}

impl AmdGpuPhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> Self {
        Self { descriptor }
    }
}

impl PlatformPhysicalDevice for AmdGpuPhysicalDevice {
    fn as_descriptor(&self) -> Option<&OwnedDescriptor> {
        Some(&self.descriptor)
    }
}

impl AsVirtGpu for AmdGpuPhysicalDevice {}
impl PhysicalDevice for AmdGpuPhysicalDevice {}
unsafe impl Send for AmdGpuPhysicalDevice {}
unsafe impl Sync for AmdGpuPhysicalDevice {}

impl GenericPhysicalDevice for AmdGpuPhysicalDevice {
    fn create_device(
        self: Arc<Self>,
        _info: &MagmaPhysicalDeviceInfo,
    ) -> MagmaGpuResult<Arc<dyn Device>> {
        Ok(Arc::new(AmdGpu::new(self)?))
    }

    fn query_memory_properties(&self) -> MagmaGpuResult<MagmaMemoryProperties> {
        let mut mem_props: MagmaMemoryProperties = Default::default();
        let mut memory_info: drm_amdgpu_memory_info = Default::default();

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_amdgpu_memory_info struct
        unsafe {
            drm_ioctl_amdgpu_info_memory(self.descriptor.as_fd(), &mut memory_info)?;
        };

        if memory_info.gtt.total_heap_size > 0 {
            mem_props.add_heap(memory_info.gtt.total_heap_size, MAGMA_HEAP_CPU_VISIBLE_BIT);
            mem_props.add_memory_type(
                MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT,
            );
            mem_props.add_memory_type(
                MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT,
            );
            mem_props.increment_heap_count();
        }

        if memory_info.vram.total_heap_size > 0 {
            mem_props.add_heap(
                memory_info.vram.total_heap_size,
                MAGMA_HEAP_DEVICE_LOCAL_BIT,
            );
            mem_props.add_memory_type(MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);
            mem_props.increment_heap_count();
        }

        if memory_info.cpu_accessible_vram.total_heap_size > 0 {
            mem_props.add_heap(
                memory_info.cpu_accessible_vram.total_heap_size,
                MAGMA_HEAP_DEVICE_LOCAL_BIT | MAGMA_HEAP_CPU_VISIBLE_BIT,
            );
            mem_props.add_memory_type(
                MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT,
            );
            mem_props.increment_heap_count();
        }

        Ok(mem_props)
    }

    fn query_queue_family_properties(&self) -> MagmaGpuResult<Vec<MagmaQueueFamilyProperties>> {
        let mut queue_families: Vec<MagmaQueueFamilyProperties> = Vec::new();
        let fd = self.descriptor.as_fd();

        let mut gfx_hw_ip: drm_amdgpu_info_hw_ip = Default::default();
        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - AMDGPU_HW_IP_GFX
        //   - drm_amdgpu_info_hw_ip struct
        unsafe {
            drm_ioctl_amdgpu_info_hw_ip(fd, AMDGPU_HW_IP_GFX, 0, &mut gfx_hw_ip)?;
        };
        if gfx_hw_ip.available_rings > 0 {
            queue_families.push(MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Graphics | MagmaQueueFlags::Compute,
                1,
            ));
        }

        let mut compute_hw_ip: drm_amdgpu_info_hw_ip = Default::default();
        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - AMDGPU_HW_IP_COMPUTE
        //   - drm_amdgpu_info_hw_ip struct
        let compute_res =
            unsafe { drm_ioctl_amdgpu_info_hw_ip(fd, AMDGPU_HW_IP_COMPUTE, 0, &mut compute_hw_ip) };
        if compute_res.is_ok() && compute_hw_ip.available_rings > 0 {
            queue_families.push(MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Compute,
                compute_hw_ip.available_rings.count_ones(),
            ));
        }

        if queue_families.is_empty() {
            return Err(MagmaGpuError::WithContext("no amdgpu queue families found"));
        }

        Ok(queue_families)
    }
}

impl AmdGpu {
    pub fn new(physical_device: Arc<dyn PhysicalDevice>) -> MagmaGpuResult<AmdGpu> {
        let mem_props = physical_device.query_memory_properties()?;

        Ok(AmdGpu {
            physical_device,
            mem_props,
        })
    }
}

impl GenericDevice for AmdGpu {
    fn get_memory_budget(&self, heap_idx: u32) -> MagmaGpuResult<MagmaHeapBudget> {
        if heap_idx >= self.mem_props.memory_heap_count {
            return Err(MagmaGpuError::WithContext("Heap Index out of bounds"));
        }

        let mut vram_gtt: drm_amdgpu_info_vram_gtt = Default::default();

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_amdgpu_memory_info_vram_gtt struct
        unsafe {
            drm_ioctl_amdgpu_info_vram_gtt(self.physical_device.as_fd().unwrap(), &mut vram_gtt)?;
        };

        let budget: u64;
        let mut usage: u64 = 0;
        let heap = &self.mem_props.memory_heaps[heap_idx as usize];

        if heap.is_device_local() && heap.is_cpu_visible() {
            budget = vram_gtt.vram_cpu_accessible_size;

            // SAFETY:
            // Valid arguments are supplied for the following arguments:
            //   - Underlying descriptor
            //   - usage
            unsafe {
                drm_ioctl_amdgpu_info_vis_vram_usage(
                    self.physical_device.as_fd().unwrap(),
                    &mut usage,
                )?;
            };
        } else if heap.is_device_local() {
            budget = vram_gtt.vram_size;

            // SAFETY:
            // Valid arguments are supplied for the following arguments:
            //   - Underlying descriptor
            //   - usage
            unsafe {
                drm_ioctl_amdgpu_info_vram_usage(
                    self.physical_device.as_fd().unwrap(),
                    &mut usage,
                )?;
            };
        } else if heap.is_cpu_visible() {
            budget = vram_gtt.gtt_size;
            // SAFETY:
            // Valid arguments are supplied for the following arguments:
            //   - Underlying descriptor
            //   - usage
            unsafe {
                drm_ioctl_amdgpu_info_gtt_usage(self.physical_device.as_fd().unwrap(), &mut usage)?;
            };
        } else {
            return Err(MagmaGpuError::Unsupported);
        }

        Ok(MagmaHeapBudget { budget, usage })
    }

    fn create_address_space(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn AddressSpace>> {
        Ok(Arc::new(AmdGpuAddressSpace {
            physical_device: self.physical_device.clone(),
        }))
    }

    fn create_queue(
        self: Arc<Self>,
        _address_space: &Arc<dyn AddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> MagmaGpuResult<Arc<dyn Queue>> {
        let queue = AmdGpuQueue::new(self.physical_device.clone(), info.priority as i32)?;
        Ok(Arc::new(queue))
    }

    fn create_buffer(
        self: Arc<Self>,
        create_info: &MagmaCreateBufferInfo,
    ) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let buf = AmdGpuBuffer::new(self.physical_device.clone(), create_info, &self.mem_props)?;
        Ok(Arc::new(buf))
    }

    fn import(self: Arc<Self>, info: MagmaImportHandleInfo) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let gem_handle = self.physical_device.import(info.handle)?;
        let buf = AmdGpuBuffer::from_existing(
            self.physical_device.clone(),
            gem_handle,
            info.size.try_into()?,
        )?;
        Ok(Arc::new(buf))
    }

    fn create_sync_obj(
        self: Arc<Self>,
        info: &MagmaCreateSyncObjInfo,
    ) -> MagmaGpuResult<Arc<dyn SyncObject>> {
        let sync_obj = DrmSyncObject::new_from_info(self.physical_device.clone(), info)?;
        Ok(Arc::new(sync_obj))
    }

    fn import_sync_obj(
        self: Arc<Self>,
        info: MagmaImportHandleInfo,
    ) -> MagmaGpuResult<Arc<dyn SyncObject>> {
        let sync_obj = DrmSyncObject::new(self.physical_device.clone(), MagmaSyncType::Binary)?;
        sync_obj.import(info.handle)?;
        Ok(Arc::new(sync_obj))
    }
}

impl Device for AmdGpu {}
impl PlatformDevice for AmdGpu {}

struct AmdGpuAddressSpace {
    physical_device: Arc<dyn PhysicalDevice>,
}

impl GenericAddressSpace for AmdGpuAddressSpace {
    fn map_buffer_gpu(
        &self,
        buffer: &Arc<dyn Buffer>,
        buffer_offset: u64,
        gpu_va: u64,
        size: u64,
        flags: MagmaGpuMapFlags,
    ) -> MagmaGpuResult<()> {
        let gem_handle = buffer.as_gem_handle().ok_or(MagmaGpuError::Unsupported)?;
        let mut map_flags = 0u32;
        if flags.contains(MagmaGpuMapFlags::Read) {
            map_flags |= AMDGPU_VM_PAGE_READABLE;
        }
        if flags.contains(MagmaGpuMapFlags::Write) {
            map_flags |= AMDGPU_VM_PAGE_WRITEABLE;
        }
        if flags.contains(MagmaGpuMapFlags::Execute) {
            map_flags |= AMDGPU_VM_PAGE_EXECUTABLE;
        }
        if map_flags == 0 {
            map_flags =
                AMDGPU_VM_PAGE_READABLE | AMDGPU_VM_PAGE_WRITEABLE | AMDGPU_VM_PAGE_EXECUTABLE;
        }

        let mut va_arg = drm_amdgpu_gem_va {
            handle: gem_handle,
            operation: AMDGPU_VA_OP_MAP,
            flags: map_flags,
            va_address: gpu_va,
            offset_in_bo: buffer_offset,
            map_size: size,
            ..Default::default()
        };

        unsafe {
            drm_ioctl_amdgpu_gem_va(self.physical_device.as_fd().unwrap(), &mut va_arg)?;
        }
        Ok(())
    }

    fn unmap_buffer_gpu(&self, gpu_va: u64, size: u64) -> MagmaGpuResult<()> {
        let mut va_arg = drm_amdgpu_gem_va {
            operation: AMDGPU_VA_OP_CLEAR,
            va_address: gpu_va,
            map_size: size,
            ..Default::default()
        };

        unsafe {
            drm_ioctl_amdgpu_gem_va(self.physical_device.as_fd().unwrap(), &mut va_arg)?;
        }
        Ok(())
    }
}

impl AddressSpace for AmdGpuAddressSpace {}

struct AmdGpuQueue {
    physical_device: Arc<dyn PhysicalDevice>,
    context_id: u32,
}

impl AmdGpuQueue {
    fn new(
        physical_device: Arc<dyn PhysicalDevice>,
        _priority: i32,
    ) -> MagmaGpuResult<AmdGpuQueue> {
        let mut ctx_arg = drm_amdgpu_ctx::default();
        ctx_arg.in_.op = AMDGPU_CTX_OP_ALLOC_CTX;

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_amdgpu_ctx struct
        let context_id: u32 = unsafe {
            drm_ioctl_amdgpu_ctx(physical_device.as_fd().unwrap(), &mut ctx_arg)?;
            ctx_arg.out.alloc.ctx_id
        };

        Ok(AmdGpuQueue {
            physical_device,
            context_id,
        })
    }
}

impl Drop for AmdGpuQueue {
    fn drop(&mut self) {
        let mut ctx_arg = drm_amdgpu_ctx::default();
        ctx_arg.in_.op = AMDGPU_CTX_OP_FREE_CTX;
        ctx_arg.in_.ctx_id = self.context_id;

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_amdgpu_ctx struct
        let result =
            unsafe { drm_ioctl_amdgpu_ctx(self.physical_device.as_fd().unwrap(), &mut ctx_arg) };
        log_status!(result);
    }
}

impl GenericQueue for AmdGpuQueue {
    fn submit_command(&self, submit_info: &MagmaSubmitInfo) -> MagmaGpuResult<()> {
        let as_info = submit_info
            .address_space_info()
            .ok_or(MagmaGpuError::Unsupported)?;

        let mut chunks: Vec<drm_amdgpu_cs_chunk> = Vec::new();

        let ib = drm_amdgpu_cs_chunk_ib {
            va_start: as_info.command_va,
            ib_bytes: as_info.length as u32,
            ip_type: AMDGPU_HW_IP_GFX,
            ..Default::default()
        };

        chunks.push(drm_amdgpu_cs_chunk {
            chunk_id: AMDGPU_CHUNK_ID_IB,
            length_dw: (std::mem::size_of::<drm_amdgpu_cs_chunk_ib>() / 4) as u32,
            chunk_data: &ib as *const _ as u64,
        });

        let mut in_syncs: Vec<drm_amdgpu_cs_chunk_sem> = Vec::new();
        let mut out_syncs: Vec<drm_amdgpu_cs_chunk_sem> = Vec::new();

        if let Some(sync_info) = submit_info.sync_info() {
            for &handle in sync_info
                .wait_sync_objs
                .iter()
                .take(sync_info.num_wait_sync_objs as usize)
            {
                if handle != 0 {
                    in_syncs.push(drm_amdgpu_cs_chunk_sem { handle });
                }
            }
            for &handle in sync_info
                .signal_sync_objs
                .iter()
                .take(sync_info.num_signal_sync_objs as usize)
            {
                if handle != 0 {
                    out_syncs.push(drm_amdgpu_cs_chunk_sem { handle });
                }
            }
        }

        if !in_syncs.is_empty() {
            chunks.push(drm_amdgpu_cs_chunk {
                chunk_id: AMDGPU_CHUNK_ID_SYNCOBJ_IN,
                length_dw: (in_syncs.len() * std::mem::size_of::<drm_amdgpu_cs_chunk_sem>() / 4)
                    as u32,
                chunk_data: in_syncs.as_ptr() as u64,
            });
        }

        if !out_syncs.is_empty() {
            chunks.push(drm_amdgpu_cs_chunk {
                chunk_id: AMDGPU_CHUNK_ID_SYNCOBJ_OUT,
                length_dw: (out_syncs.len() * std::mem::size_of::<drm_amdgpu_cs_chunk_sem>() / 4)
                    as u32,
                chunk_data: out_syncs.as_ptr() as u64,
            });
        }

        let chunk_ptrs: Vec<u64> = chunks.iter().map(|c| c as *const _ as u64).collect();
        let mut cs = drm_amdgpu_cs::default();
        cs.in_.ctx_id = self.context_id;
        cs.in_.num_chunks = chunks.len() as u32;
        cs.in_.chunks = chunk_ptrs.as_ptr() as u64;

        unsafe {
            drm_ioctl_amdgpu_cs(self.physical_device.as_fd().unwrap(), &mut cs)?;
        }
        Ok(())
    }
}

impl Queue for AmdGpuQueue {}

impl AmdGpuBuffer {
    fn new(
        physical_device: Arc<dyn PhysicalDevice>,
        create_info: &MagmaCreateBufferInfo,
        mem_props: &MagmaMemoryProperties,
    ) -> MagmaGpuResult<AmdGpuBuffer> {
        let mut gem_create_in: drm_amdgpu_gem_create_in = Default::default();
        let mut gem_create: drm_amdgpu_gem_create = Default::default();

        let memory_type = mem_props.get_memory_type(create_info.memory_type_idx);

        gem_create_in.bo_size = create_info.size;
        // FIXME: gpu_info.pte_fragment_size, alignment
        // Need GPU topology crate
        gem_create_in.alignment = create_info.alignment as u64;

        // Goal: An explicit sync world + discardable world only.
        gem_create_in.domain_flags |= AMDGPU_GEM_CREATE_EXPLICIT_SYNC as u64;
        gem_create_in.domain_flags |= AMDGPU_GEM_CREATE_DISCARDABLE as u64;

        if memory_type.is_coherent() {
            gem_create_in.domain_flags |= AMDGPU_GEM_CREATE_CPU_GTT_USWC as u64;
        } else {
            gem_create_in.domain_flags |= AMDGPU_GEM_CREATE_NO_CPU_ACCESS as u64;
        }

        if memory_type.is_protected() {
            gem_create_in.domain_flags |= AMDGPU_GEM_CREATE_ENCRYPTED as u64;
        }

        // Should these be "heaps" of zero size?
        if create_info.vendor_flags & MAGMA_BUFFER_FLAG_AMD_OA != 0 {
            gem_create_in.domains |= AMDGPU_GEM_DOMAIN_OA as u64
        } else if create_info.vendor_flags & MAGMA_BUFFER_FLAG_AMD_GDS != 0 {
            gem_create_in.domains |= AMDGPU_GEM_DOMAIN_GDS as u64;
        } else if memory_type.is_device_local() {
            gem_create_in.domains |= AMDGPU_GEM_DOMAIN_VRAM as u64;
        } else {
            gem_create_in.domains |= AMDGPU_GEM_DOMAIN_GTT as u64;
        }

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_amdgpu_gem_create_args
        let gem_handle = unsafe {
            gem_create.in_ = gem_create_in;
            drm_ioctl_amdgpu_gem_create(physical_device.as_fd().unwrap(), &mut gem_create)?;
            gem_create.out.handle
        };

        Ok(AmdGpuBuffer {
            physical_device,
            gem_handle,
            size: create_info.size.try_into()?,
        })
    }

    fn from_existing(
        physical_device: Arc<dyn PhysicalDevice>,
        gem_handle: u32,
        size: usize,
    ) -> MagmaGpuResult<AmdGpuBuffer> {
        Ok(AmdGpuBuffer {
            physical_device,
            gem_handle,
            size,
        })
    }
}

impl GenericBuffer for AmdGpuBuffer {
    fn map(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn MappedRegion>> {
        let mut gem_mmap: drm_amdgpu_gem_mmap = Default::default();

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_amdgpu_gem_mmap
        let offset = unsafe {
            gem_mmap.in_.handle = self.gem_handle;
            drm_ioctl_amdgpu_gem_mmap(self.physical_device.as_fd().unwrap(), &mut gem_mmap)?;
            gem_mmap.out.addr_ptr
        };

        let mapping = self.physical_device.cpu_map(offset, self.size)?;
        Ok(Arc::new(mapping))
    }

    fn export(&self) -> MagmaGpuResult<MagmaGpuHandle> {
        self.physical_device.export(self.gem_handle)
    }

    fn invalidate(
        &self,
        _sync_flags: u64,
        _ranges: &[MagmaMappedMemoryRange],
    ) -> MagmaGpuResult<()> {
        Err(MagmaGpuError::Unsupported)
    }

    fn flush(&self, _sync_flags: u64, _ranges: &[MagmaMappedMemoryRange]) -> MagmaGpuResult<()> {
        Err(MagmaGpuError::Unsupported)
    }

    fn as_gem_handle(&self) -> Option<u32> {
        Some(self.gem_handle)
    }
}

impl Drop for AmdGpuBuffer {
    fn drop(&mut self) {
        // GEM close
    }
}

impl Buffer for AmdGpuBuffer {}

unsafe impl Send for AmdGpu {}
unsafe impl Sync for AmdGpu {}

unsafe impl Send for AmdGpuAddressSpace {}
unsafe impl Sync for AmdGpuAddressSpace {}

unsafe impl Send for AmdGpuQueue {}
unsafe impl Sync for AmdGpuQueue {}

unsafe impl Send for AmdGpuBuffer {}
unsafe impl Sync for AmdGpuBuffer {}
