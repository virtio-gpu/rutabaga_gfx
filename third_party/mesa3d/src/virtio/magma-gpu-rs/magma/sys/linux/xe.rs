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
use crate::sys::linux::drm::DrmSyncObject;

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

use zerocopy::FromZeros;
use zerocopy::KnownLayout;

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
use crate::magma_defines::MagmaSyncType;
use crate::magma_defines::MAGMA_BUFFER_FLAG_SCANOUT;
use crate::magma_defines::MAGMA_HEAP_CPU_VISIBLE_BIT;
use crate::magma_defines::MAGMA_HEAP_DEVICE_LOCAL_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT;
use crate::protocol::MagmaGpuMapFlags;

use crate::sys::linux::bindings::drm_bindings::DRM_COMMAND_BASE;
use crate::sys::linux::bindings::drm_bindings::DRM_IOCTL_BASE;
use crate::sys::linux::bindings::xe_bindings::*;
use crate::sys::linux::PlatformDevice;
use crate::sys::linux::PlatformPhysicalDevice;

// This information is also useful to the system side of a driver.  Should be separated
// into it's own crate or module.
const GEN12_IDS: [u16; 50] = [
    0x4c8a, 0x4c8b, 0x4c8c, 0x4c90, 0x4c9a, 0x4680, 0x4681, 0x4682, 0x4683, 0x4688, 0x4689, 0x4690,
    0x4691, 0x4692, 0x4693, 0x4698, 0x4699, 0x4626, 0x4628, 0x462a, 0x46a0, 0x46a1, 0x46a2, 0x46a3,
    0x46a6, 0x46a8, 0x46aa, 0x46b0, 0x46b1, 0x46b2, 0x46b3, 0x46c0, 0x46c1, 0x46c2, 0x46c3, 0x9A40,
    0x9A49, 0x9A59, 0x9A60, 0x9A68, 0x9A70, 0x9A78, 0x9AC0, 0x9AC9, 0x9AD9, 0x9AF8, 0x4905, 0x4906,
    0x4907, 0x4908,
];

const ADLP_IDS: [u16; 23] = [
    0x46A0, 0x46A1, 0x46A2, 0x46A3, 0x46A6, 0x46A8, 0x46AA, 0x462A, 0x4626, 0x4628, 0x46B0, 0x46B1,
    0x46B2, 0x46B3, 0x46C0, 0x46C1, 0x46C2, 0x46C3, 0x46D0, 0x46D1, 0x46D2, 0x46D3, 0x46D4,
];

const RPLP_IDS: [u16; 10] = [
    0xA720, 0xA721, 0xA7A0, 0xA7A1, 0xA7A8, 0xA7A9, 0xA7AA, 0xA7AB, 0xA7AC, 0xA7AD,
];

const MTL_IDS: [u16; 5] = [0x7D40, 0x7D60, 0x7D45, 0x7D55, 0x7DD5];

const LNL_IDS: [u16; 3] = [0x6420, 0x64A0, 0x64B0];

const PTL_IDS: [u16; 8] = [
    0xB080, 0xB081, 0xB082, 0xB083, 0xB08F, 0xB090, 0xB0A0, 0xB0B0,
];

const DG2_IDS: [u16; 26] = [
    0x5690, 0x5691, 0x5692, 0x5693, 0x5694, 0x5695, 0x5696, 0x5697, 0x56a0, 0x56a1, 0x56a2, 0x56a3,
    0x56a4, 0x56a5, 0x56a6, 0x56b0, 0x56b1, 0x56b2, 0x56b3, 0x56ba, 0x56bb, 0x56bc, 0x56bd, 0x56be,
    0x56bf, 0x56c0,
];

ioctl_readwrite!(
    drm_ioctl_xe_device_query,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_DEVICE_QUERY,
    drm_xe_device_query
);

ioctl_readwrite!(
    drm_ioctl_xe_gem_create,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_GEM_CREATE,
    drm_xe_gem_create
);

ioctl_readwrite!(
    drm_ioctl_xe_gem_mmap_offset,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_GEM_MMAP_OFFSET,
    drm_xe_gem_mmap_offset
);

ioctl_readwrite!(
    drm_ioctl_xe_vm_create,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_VM_CREATE,
    drm_xe_vm_create
);

ioctl_write_ptr!(
    drm_ioctl_xe_vm_destroy,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_VM_DESTROY,
    drm_xe_vm_destroy
);

ioctl_readwrite!(
    drm_ioctl_xe_vm_bind,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_VM_BIND,
    drm_xe_vm_bind
);

ioctl_readwrite!(
    drm_ioctl_xe_exec_queue_create,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_EXEC_QUEUE_CREATE,
    drm_xe_exec_queue_create
);

ioctl_write_ptr!(
    drm_ioctl_xe_exec_queue_destroy,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_EXEC_QUEUE_DESTROY,
    drm_xe_exec_queue_destroy
);

ioctl_write_ptr!(
    drm_ioctl_xe_exec,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_EXEC,
    drm_xe_exec
);

ioctl_readwrite!(
    drm_ioctl_xe_exec_queue_get_property,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_EXEC_QUEUE_GET_PROPERTY,
    drm_xe_exec_queue_get_property
);

#[derive(Default)]
struct XeMemoryInfo {
    sysmem_size: u64,
    vram_size: u64,
    sysmem_used: u64,
    vram_used: u64,
    vram_cpu_visible_size: u64,
    vram_cpu_visible_used: u64,
    sysmem_instance: u16,
    vram_instance: u16,
}

pub struct Xe {
    physical_device: Arc<dyn PhysicalDevice>,
    _gtt_size: u64,
    _mem_alignment: u64,
    mem_props: MagmaMemoryProperties,
    sysmem_instance: u16,
    vram_instance: u16,
}

struct XeBuffer {
    physical_device: Arc<dyn PhysicalDevice>,
    gem_handle: u32,
    size: usize,
}

/// Determines and sets the graphics version of the Intel device based on its ID.
fn determine_graphics_version(pci_device_id: u16) -> MagmaGpuResult<u32> {
    let mut graphics_version = 0;
    if ADLP_IDS.contains(&pci_device_id) {
        graphics_version = 12;
    }

    if RPLP_IDS.contains(&pci_device_id) {
        graphics_version = 12;
    }

    if MTL_IDS.contains(&pci_device_id) {
        graphics_version = 12;
    }

    if LNL_IDS.contains(&pci_device_id) {
        graphics_version = 20;
    }

    if PTL_IDS.contains(&pci_device_id) {
        graphics_version = 20;
    }

    if GEN12_IDS.contains(&pci_device_id) {
        graphics_version = 12;
    }

    if DG2_IDS.contains(&pci_device_id) {
        graphics_version = 12;
    }

    if graphics_version != 0 {
        Ok(graphics_version)
    } else {
        // Fall back to Gen12 as Xe driver only supports Gen12+
        Ok(12)
    }
}

fn xe_device_query<T, S, H>(fd: BorrowedFd<'_>, query_id: u32) -> MagmaGpuResult<Box<T>>
where
    T: ?Sized + FromZeros + KnownLayout<PointerMetadata = usize>,
{
    let mut device_query = drm_xe_device_query {
        query: query_id,
        ..Default::default()
    };

    // Ask kernel for the size
    unsafe {
        drm_ioctl_xe_device_query(fd, &mut device_query)?;
    }

    let total_size = device_query.size as usize;
    let header_size = std::mem::size_of::<H>();
    let element_size = std::mem::size_of::<S>();
    let count = total_size
        .saturating_sub(header_size)
        .div_ceil(element_size);

    // Allocate Box<T> directly for the dynamically size type (DST)
    let mut query_data = T::new_box_zeroed_with_elems(count)
        .map_err(|_| MagmaGpuError::WithContext("Failed to allocate query data"))?;

    // Tell kernel to write data into the allocated DST Box
    device_query.size = total_size as u32;
    device_query.data = &mut *query_data as *mut T as *mut () as u64;

    unsafe {
        drm_ioctl_xe_device_query(fd, &mut device_query)?;
    }

    Ok(query_data)
}

fn xe_query_memory_regions(fd: BorrowedFd<'_>) -> MagmaGpuResult<XeMemoryInfo> {
    let mut memory_info: XeMemoryInfo = Default::default();

    let query_mem_regions = xe_device_query::<
        drm_xe_query_mem_regions<[drm_xe_mem_region]>,
        drm_xe_mem_region,
        drm_xe_query_mem_regions<[drm_xe_mem_region; 0]>,
    >(fd, DRM_XE_DEVICE_QUERY_MEM_REGIONS)?;

    let num_regions = std::cmp::min(
        query_mem_regions.num_mem_regions as usize,
        query_mem_regions.mem_regions.len(),
    );
    let mem_regions = &query_mem_regions.mem_regions[..num_regions];
    for region in mem_regions {
        match region.mem_class as u32 {
            DRM_XE_MEM_REGION_CLASS_SYSMEM => {
                if memory_info.sysmem_size != 0 {
                    return Err(MagmaGpuError::WithContext("sysmem_size should not be set"));
                }

                // this should really use sysconf(_SC_PHYS_PAGES) * sysconf(_SC_PAGE_SIZE) for the
                // host-visible heap.  rustix has get_page_size(), but not get_num_pages..
                memory_info.sysmem_size = region.total_size;
                memory_info.sysmem_used = region.used;
                memory_info.sysmem_instance = region.instance;
            }
            DRM_XE_MEM_REGION_CLASS_VRAM => {
                if memory_info.vram_size != 0 || memory_info.vram_cpu_visible_size != 0 {
                    return Err(MagmaGpuError::WithContext("one vram value should be zero"));
                }

                memory_info.vram_cpu_visible_size = region.cpu_visible_size;
                memory_info.vram_size = region.total_size - region.cpu_visible_size;
                memory_info.vram_cpu_visible_used = region.cpu_visible_used;
                memory_info.vram_used = region.used - region.cpu_visible_used;
                memory_info.vram_instance = region.instance;
            }
            _ => return Err(MagmaGpuError::Unsupported),
        }
    }

    Ok(memory_info)
}

fn xe_query_memory_properties(fd: BorrowedFd<'_>) -> MagmaGpuResult<MagmaMemoryProperties> {
    let mut mem_props: MagmaMemoryProperties = Default::default();
    let memory_info = xe_query_memory_regions(fd)?;
    if memory_info.sysmem_size != 0 {
        // Non-LLC case ignored.
        mem_props.add_heap(memory_info.sysmem_size, MAGMA_HEAP_CPU_VISIBLE_BIT);
        mem_props.add_memory_type(
            MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT,
        );

        mem_props.increment_heap_count();
    }

    if memory_info.vram_cpu_visible_size != 0 {
        mem_props.add_heap(
            memory_info.vram_cpu_visible_size,
            MAGMA_HEAP_CPU_VISIBLE_BIT | MAGMA_HEAP_DEVICE_LOCAL_BIT,
        );
        mem_props.add_memory_type(
            MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT,
        );

        mem_props.increment_heap_count();
    }

    if memory_info.vram_size != 0 {
        mem_props.add_heap(memory_info.vram_size, MAGMA_HEAP_DEVICE_LOCAL_BIT);
        mem_props.add_memory_type(MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);
        mem_props.increment_heap_count();
    }

    Ok(mem_props)
}

#[derive(Debug)]
pub struct XePhysicalDevice {
    descriptor: OwnedDescriptor,
}

impl XePhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> Self {
        Self { descriptor }
    }
}

impl PlatformPhysicalDevice for XePhysicalDevice {
    fn as_descriptor(&self) -> Option<&OwnedDescriptor> {
        Some(&self.descriptor)
    }
}

impl AsVirtGpu for XePhysicalDevice {}
impl PhysicalDevice for XePhysicalDevice {}
unsafe impl Send for XePhysicalDevice {}
unsafe impl Sync for XePhysicalDevice {}

impl GenericPhysicalDevice for XePhysicalDevice {
    fn create_device(
        self: Arc<Self>,
        info: &MagmaPhysicalDeviceInfo,
    ) -> MagmaGpuResult<Arc<dyn Device>> {
        Ok(Arc::new(Xe::new(self, info)?))
    }

    fn query_memory_properties(&self) -> MagmaGpuResult<MagmaMemoryProperties> {
        xe_query_memory_properties(self.descriptor.as_fd())
    }

    fn query_queue_family_properties(&self) -> MagmaGpuResult<Vec<MagmaQueueFamilyProperties>> {
        Ok(vec![
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Graphics
                    | MagmaQueueFlags::Compute
                    | MagmaQueueFlags::Transfer
                    | MagmaQueueFlags::SparseBinding
                    | MagmaQueueFlags::Protected,
                1,
            ),
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Compute
                    | MagmaQueueFlags::Transfer
                    | MagmaQueueFlags::SparseBinding,
                1,
            ),
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Transfer | MagmaQueueFlags::Protected,
                1,
            ),
        ])
    }
}

impl Xe {
    pub fn new(
        physical_device: Arc<dyn PhysicalDevice>,
        info: &MagmaPhysicalDeviceInfo,
    ) -> MagmaGpuResult<Xe> {
        let fd = physical_device
            .as_fd()
            .ok_or(MagmaGpuError::WithContext("no fd"))?;
        let _graphics_version = determine_graphics_version(info.device_id)?;

        let query_config = xe_device_query::<
            drm_xe_query_config<[__u64]>,
            __u64,
            drm_xe_query_config<[__u64; 0]>,
        >(fd, DRM_XE_DEVICE_QUERY_CONFIG)?;

        let num_params = std::cmp::min(query_config.num_params as usize, query_config.info.len());
        let config = &query_config.info[..num_params];
        let _config_len = config.len();

        let gtt_size = 1u64 << config[DRM_XE_QUERY_CONFIG_VA_BITS as usize];
        let mem_alignment = config[DRM_XE_QUERY_CONFIG_MIN_ALIGNMENT as usize];

        let memory_info = xe_query_memory_regions(fd)?;
        let mem_props = if info.memory_properties.memory_heap_count > 0 {
            info.memory_properties
        } else {
            physical_device.query_memory_properties()?
        };

        Ok(Xe {
            physical_device,
            _gtt_size: gtt_size,
            _mem_alignment: mem_alignment,
            mem_props,
            sysmem_instance: memory_info.sysmem_instance,
            vram_instance: memory_info.vram_instance,
        })
    }
}

impl GenericDevice for Xe {
    fn get_memory_budget(&self, heap_idx: u32) -> MagmaGpuResult<MagmaHeapBudget> {
        if heap_idx >= self.mem_props.memory_heap_count {
            return Err(MagmaGpuError::WithContext("Heap Index out of bounds"));
        }

        let fd = self
            .physical_device
            .as_fd()
            .ok_or(MagmaGpuError::WithContext("no fd"))?;
        let memory_info = xe_query_memory_regions(fd)?;
        let heap = &self.mem_props.memory_heaps[heap_idx as usize];

        let (budget, usage) = if heap.is_device_local() && heap.is_cpu_visible() {
            (
                memory_info.vram_cpu_visible_size,
                memory_info.vram_cpu_visible_used,
            )
        } else if heap.is_device_local() {
            (memory_info.vram_size, memory_info.vram_used)
        } else if heap.is_cpu_visible() {
            (memory_info.sysmem_size, memory_info.sysmem_used)
        } else {
            return Err(MagmaGpuError::Unsupported);
        };

        Ok(MagmaHeapBudget { budget, usage })
    }

    fn create_address_space(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn AddressSpace>> {
        let addr_space = XeAddressSpace::new(self.physical_device.clone(), 0)?;
        Ok(Arc::new(addr_space))
    }

    fn create_queue(
        self: Arc<Self>,
        address_space: &Arc<dyn AddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> MagmaGpuResult<Arc<dyn Queue>> {
        let queue = XeQueue::new(self.physical_device.clone(), address_space, info)?;
        Ok(Arc::new(queue))
    }

    fn create_buffer(
        self: Arc<Self>,
        create_info: &MagmaCreateBufferInfo,
    ) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let buf = XeBuffer::new(
            self.physical_device.clone(),
            create_info,
            &self.mem_props,
            self.sysmem_instance,
            self.vram_instance,
        )?;
        Ok(Arc::new(buf))
    }

    fn import(self: Arc<Self>, info: MagmaImportHandleInfo) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let gem_handle = self.physical_device.import(info.handle)?;
        let buf = XeBuffer::from_existing(
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

impl PlatformDevice for Xe {}
impl Device for Xe {}

struct XeAddressSpace {
    physical_device: Arc<dyn PhysicalDevice>,
    vm_id: u32,
}

impl XeAddressSpace {
    fn new(
        physical_device: Arc<dyn PhysicalDevice>,
        _priority: i32,
    ) -> MagmaGpuResult<XeAddressSpace> {
        let mut vm_create = drm_xe_vm_create {
            flags: DRM_XE_VM_CREATE_FLAG_SCRATCH_PAGE,
            ..Default::default()
        };

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_xe_vm_create struct
        unsafe {
            drm_ioctl_xe_vm_create(physical_device.as_fd().unwrap(), &mut vm_create)?;
        };

        Ok(XeAddressSpace {
            physical_device,
            vm_id: vm_create.vm_id,
        })
    }
}

impl Drop for XeAddressSpace {
    fn drop(&mut self) {
        let destroy = drm_xe_vm_destroy {
            vm_id: self.vm_id,
            ..Default::default()
        };

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_xe_vm_destroy struct
        let result =
            unsafe { drm_ioctl_xe_vm_destroy(self.physical_device.as_fd().unwrap(), &destroy) };
        log_status!(result);
    }
}

impl GenericAddressSpace for XeAddressSpace {
    fn as_vm_id(&self) -> Option<u32> {
        Some(self.vm_id)
    }

    fn map_buffer_gpu(
        &self,
        buffer: &Arc<dyn Buffer>,
        buffer_offset: u64,
        gpu_va: u64,
        size: u64,
        _flags: MagmaGpuMapFlags,
    ) -> MagmaGpuResult<()> {
        let gem_handle = buffer.as_gem_handle().ok_or(MagmaGpuError::Unsupported)?;
        let bind_op = drm_xe_vm_bind_op {
            op: DRM_XE_VM_BIND_OP_MAP,
            flags: 0,
            addr: gpu_va,
            range: size,
            obj: gem_handle,
            __bindgen_anon_1: drm_xe_vm_bind_op__bindgen_ty_1 {
                obj_offset: buffer_offset,
            },
            ..Default::default()
        };

        let mut vm_bind = drm_xe_vm_bind {
            vm_id: self.vm_id,
            num_binds: 1,
            __bindgen_anon_1: drm_xe_vm_bind__bindgen_ty_1 { bind: bind_op },
            ..Default::default()
        };

        unsafe {
            drm_ioctl_xe_vm_bind(self.physical_device.as_fd().unwrap(), &mut vm_bind)?;
        }

        Ok(())
    }

    fn unmap_buffer_gpu(&self, gpu_va: u64, size: u64) -> MagmaGpuResult<()> {
        let bind_op = drm_xe_vm_bind_op {
            op: DRM_XE_VM_BIND_OP_UNMAP,
            flags: 0,
            addr: gpu_va,
            range: size,
            ..Default::default()
        };

        let mut vm_bind = drm_xe_vm_bind {
            vm_id: self.vm_id,
            num_binds: 1,
            __bindgen_anon_1: drm_xe_vm_bind__bindgen_ty_1 { bind: bind_op },
            ..Default::default()
        };

        unsafe {
            drm_ioctl_xe_vm_bind(self.physical_device.as_fd().unwrap(), &mut vm_bind)?;
        }

        Ok(())
    }
}

impl AddressSpace for XeAddressSpace {}

struct XeQueue {
    physical_device: Arc<dyn PhysicalDevice>,
    exec_queue_id: u32,
    vm_id: u32,
    is_bind_queue: bool,
}

impl XeQueue {
    fn new(
        physical_device: Arc<dyn PhysicalDevice>,
        address_space: &Arc<dyn AddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> MagmaGpuResult<XeQueue> {
        let vm_id = address_space.as_vm_id().ok_or(MagmaGpuError::Unsupported)?;
        let is_bind_queue = (info.flags & MagmaQueueFlags::SparseBinding.bits()) != 0;

        let engine_class = if is_bind_queue {
            DRM_XE_ENGINE_CLASS_VM_BIND as u16
        } else if info.queue_family_idx == 1 {
            DRM_XE_ENGINE_CLASS_COMPUTE as u16
        } else if info.queue_family_idx == 2 {
            DRM_XE_ENGINE_CLASS_COPY as u16
        } else {
            DRM_XE_ENGINE_CLASS_RENDER as u16
        };

        let instance = drm_xe_engine_class_instance {
            engine_class,
            engine_instance: 0,
            gt_id: 0,
            pad: 0,
        };
        let mut create = drm_xe_exec_queue_create {
            instances: &instance as *const _ as u64,
            width: 1,
            num_placements: 1,
            vm_id,
            ..Default::default()
        };
        unsafe {
            drm_ioctl_xe_exec_queue_create(physical_device.as_fd().unwrap(), &mut create)?;
        }
        Ok(XeQueue {
            physical_device,
            exec_queue_id: create.exec_queue_id,
            vm_id,
            is_bind_queue,
        })
    }
}

impl Drop for XeQueue {
    fn drop(&mut self) {
        if let Some(fd) = self.physical_device.as_fd() {
            let destroy = drm_xe_exec_queue_destroy {
                exec_queue_id: self.exec_queue_id,
                ..Default::default()
            };
            let result = unsafe { drm_ioctl_xe_exec_queue_destroy(fd, &destroy) };
            log_status!(result);
        }
    }
}

impl GenericQueue for XeQueue {
    fn submit_command(&self, submit_info: &MagmaSubmitInfo) -> MagmaGpuResult<()> {
        let fd = self.physical_device.as_fd().unwrap();

        let mut syncs: Vec<drm_xe_sync> = Vec::new();
        if let Some(sync_info) = submit_info.sync_info() {
            for &handle in sync_info
                .wait_sync_objs
                .iter()
                .take(sync_info.num_wait_sync_objs as usize)
            {
                if handle != 0 {
                    let s = drm_xe_sync {
                        type_: DRM_XE_SYNC_TYPE_SYNCOBJ,
                        timeline_value: 0,
                        flags: 0,
                        __bindgen_anon_1: drm_xe_sync__bindgen_ty_1 { handle },
                        ..Default::default()
                    };
                    syncs.push(s);
                }
            }
            for &handle in sync_info
                .signal_sync_objs
                .iter()
                .take(sync_info.num_signal_sync_objs as usize)
            {
                if handle != 0 {
                    let s = drm_xe_sync {
                        type_: DRM_XE_SYNC_TYPE_SYNCOBJ,
                        timeline_value: 0,
                        flags: DRM_XE_SYNC_FLAG_SIGNAL,
                        __bindgen_anon_1: drm_xe_sync__bindgen_ty_1 { handle },
                        ..Default::default()
                    };
                    syncs.push(s);
                }
            }
        }

        if self.is_bind_queue {
            let mut vm_bind = drm_xe_vm_bind {
                vm_id: self.vm_id,
                exec_queue_id: self.exec_queue_id,
                num_binds: 0,
                num_syncs: syncs.len() as u32,
                syncs: syncs.as_ptr() as u64,
                ..Default::default()
            };
            unsafe {
                drm_ioctl_xe_vm_bind(fd, &mut vm_bind)?;
            }
        } else {
            let as_info = submit_info
                .address_space_info()
                .ok_or(MagmaGpuError::Unsupported)?;

            let num_batch_buffer = if as_info.command_va != 0 && as_info.length != 0 {
                1
            } else {
                0
            };
            let address = if num_batch_buffer > 0 {
                as_info.command_va
            } else {
                0
            };

            let exec = drm_xe_exec {
                exec_queue_id: self.exec_queue_id,
                num_batch_buffer,
                address,
                num_syncs: syncs.len() as u32,
                syncs: syncs.as_ptr() as u64,
                ..Default::default()
            };
            unsafe {
                drm_ioctl_xe_exec(fd, &exec)?;
            }
        }
        Ok(())
    }

    fn check_status(&self) -> MagmaResult<()> {
        let mut prop = drm_xe_exec_queue_get_property {
            extensions: 0,
            exec_queue_id: self.exec_queue_id,
            property: DRM_XE_EXEC_QUEUE_GET_PROPERTY_BAN,
            value: 0,
            reserved: [0; 2],
        };
        let fd = self
            .physical_device
            .as_fd()
            .ok_or(MagmaError::InternalError)?;
        unsafe {
            drm_ioctl_xe_exec_queue_get_property(fd, &mut prop)
                .map_err(|_| MagmaError::InternalError)?;
        }
        if prop.value != 0 {
            return Err(MagmaError::ContextKilled);
        }
        Ok(())
    }
}

impl Queue for XeQueue {}

impl XeBuffer {
    fn new(
        physical_device: Arc<dyn PhysicalDevice>,
        create_info: &MagmaCreateBufferInfo,
        mem_props: &MagmaMemoryProperties,
        sysmem_instance: u16,
        vram_instance: u16,
    ) -> MagmaGpuResult<XeBuffer> {
        let mut gem_create: drm_xe_gem_create = Default::default();
        let mut pxp_ext: drm_xe_ext_set_property = Default::default();

        gem_create.size = create_info.size;
        let memory_type = mem_props.get_memory_type(create_info.memory_type_idx);
        let memory_heap = mem_props.get_memory_heap(memory_type.heap_idx);

        let is_scanout = (create_info.common_flags & MAGMA_BUFFER_FLAG_SCANOUT) != 0;
        if is_scanout {
            gem_create.flags |= DRM_XE_GEM_CREATE_FLAG_SCANOUT;
            gem_create.cpu_caching = DRM_XE_GEM_CPU_CACHING_WC as u16;
        } else if memory_type.is_cached() {
            gem_create.cpu_caching = DRM_XE_GEM_CPU_CACHING_WB as u16;
        } else {
            gem_create.cpu_caching = DRM_XE_GEM_CPU_CACHING_WC as u16;
        }

        if memory_heap.is_cpu_visible() && memory_heap.is_device_local() {
            gem_create.flags |= DRM_XE_GEM_CREATE_FLAG_NEEDS_VISIBLE_VRAM;
            gem_create.placement |= 1 << sysmem_instance;
            gem_create.placement |= 1 << vram_instance;
        } else if memory_heap.is_device_local() {
            gem_create.placement |= 1 << vram_instance;
        } else if memory_heap.is_cpu_visible() {
            gem_create.placement |= 1 << sysmem_instance;
        }

        if memory_type.is_protected() {
            pxp_ext.base.name = DRM_XE_GEM_CREATE_EXTENSION_SET_PROPERTY;
            pxp_ext.property = DRM_XE_GEM_CREATE_SET_PROPERTY_PXP_TYPE;
            pxp_ext.__bindgen_anon_1.value = DRM_XE_PXP_TYPE_HWDRM as u64;
            gem_create.extensions = &pxp_ext as *const drm_xe_ext_set_property as u64;
        }

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_amdgpu_gem_create_args
        unsafe {
            drm_ioctl_xe_gem_create(physical_device.as_fd().unwrap(), &mut gem_create)?;
        };

        Ok(XeBuffer {
            physical_device,
            gem_handle: gem_create.handle,
            size: create_info.size.try_into()?,
        })
    }

    fn from_existing(
        physical_device: Arc<dyn PhysicalDevice>,
        gem_handle: u32,
        size: usize,
    ) -> MagmaGpuResult<XeBuffer> {
        Ok(XeBuffer {
            physical_device,
            gem_handle,
            size,
        })
    }
}

impl GenericBuffer for XeBuffer {
    fn map(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn MappedRegion>> {
        let mut xe_offset: drm_xe_gem_mmap_offset = Default::default();

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_xe_gem_mmap_offset
        let offset = unsafe {
            xe_offset.handle = self.gem_handle;
            drm_ioctl_xe_gem_mmap_offset(self.physical_device.as_fd().unwrap(), &mut xe_offset)?;
            xe_offset.offset
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

impl Drop for XeBuffer {
    fn drop(&mut self) {
        self.physical_device.close(self.gem_handle)
    }
}

impl Buffer for XeBuffer {}

unsafe impl Send for Xe {}
unsafe impl Sync for Xe {}

unsafe impl Send for XeAddressSpace {}
unsafe impl Sync for XeAddressSpace {}

unsafe impl Send for XeQueue {}
unsafe impl Sync for XeQueue {}

unsafe impl Send for XeBuffer {}
unsafe impl Sync for XeBuffer {}
