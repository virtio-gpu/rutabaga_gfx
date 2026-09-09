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
use crate::magma_defines::MAGMA_HEAP_CPU_VISIBLE_BIT;
use crate::magma_defines::MAGMA_HEAP_DEVICE_LOCAL_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT;
use crate::magma_defines::MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT;
use crate::protocol::MagmaGpuMapFlags;

use crate::sys::linux::bindings::drm_bindings::DRM_COMMAND_BASE;
use crate::sys::linux::bindings::drm_bindings::DRM_IOCTL_BASE;
use crate::sys::linux::bindings::panthor_bindings::*;
use crate::sys::linux::PlatformDevice;
use crate::sys::linux::PlatformPhysicalDevice;

ioctl_readwrite!(
    drm_ioctl_panthor_dev_query,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_DEV_QUERY,
    drm_panthor_dev_query
);

ioctl_readwrite!(
    drm_ioctl_panthor_vm_create,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_VM_CREATE,
    drm_panthor_vm_create
);

ioctl_write_ptr!(
    drm_ioctl_panthor_vm_destroy,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_VM_DESTROY,
    drm_panthor_vm_destroy
);

ioctl_readwrite!(
    drm_ioctl_panthor_vm_bind,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_VM_BIND,
    drm_panthor_vm_bind
);

ioctl_readwrite!(
    drm_ioctl_panthor_vm_get_state,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_VM_GET_STATE,
    drm_panthor_vm_get_state
);

ioctl_readwrite!(
    drm_ioctl_panthor_bo_create,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_BO_CREATE,
    drm_panthor_bo_create
);

ioctl_readwrite!(
    drm_ioctl_panthor_bo_mmap_offset,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_BO_MMAP_OFFSET,
    drm_panthor_bo_mmap_offset
);

ioctl_readwrite!(
    drm_ioctl_panthor_group_create,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_GROUP_CREATE,
    drm_panthor_group_create
);

ioctl_write_ptr!(
    drm_ioctl_panthor_group_destroy,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_GROUP_DESTROY,
    drm_panthor_group_destroy
);

ioctl_readwrite!(
    drm_ioctl_panthor_group_submit,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_GROUP_SUBMIT,
    drm_panthor_group_submit
);

ioctl_readwrite!(
    drm_ioctl_panthor_group_get_state,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_GROUP_GET_STATE,
    drm_panthor_group_get_state
);

ioctl_readwrite!(
    drm_ioctl_panthor_bo_sync,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_PANTHOR_BO_SYNC,
    drm_panthor_bo_sync
);

fn panthor_query_gpu_info(fd: BorrowedFd<'_>) -> MagmaGpuResult<drm_panthor_gpu_info> {
    let mut gpu_info: drm_panthor_gpu_info = Default::default();
    let mut query = drm_panthor_dev_query {
        type_: DRM_PANTHOR_DEV_QUERY_GPU_INFO,
        size: std::mem::size_of::<drm_panthor_gpu_info>() as u32,
        pointer: &mut gpu_info as *mut _ as u64,
    };
    unsafe {
        drm_ioctl_panthor_dev_query(fd, &mut query)?;
    }
    Ok(gpu_info)
}

fn panthor_query_csif_info(fd: BorrowedFd<'_>) -> MagmaGpuResult<drm_panthor_csif_info> {
    let mut csif_info: drm_panthor_csif_info = Default::default();
    let mut query = drm_panthor_dev_query {
        type_: DRM_PANTHOR_DEV_QUERY_CSIF_INFO,
        size: std::mem::size_of::<drm_panthor_csif_info>() as u32,
        pointer: &mut csif_info as *mut _ as u64,
    };
    unsafe {
        drm_ioctl_panthor_dev_query(fd, &mut query)?;
    }
    Ok(csif_info)
}

fn panthor_query_memory_properties(gpu_info: &drm_panthor_gpu_info) -> MagmaMemoryProperties {
    let mut mem_props: MagmaMemoryProperties = Default::default();
    let heap_size = 4 * 1024 * 1024 * 1024; // 4 GiB default unified memory heap
    mem_props.add_heap(
        heap_size,
        MAGMA_HEAP_DEVICE_LOCAL_BIT | MAGMA_HEAP_CPU_VISIBLE_BIT,
    );

    // Type 0: Device-local only (non-host visible, for GPU-only/protected/imported allocations)
    mem_props.add_memory_type(MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);

    // Type 1: Host-visible & host-coherent (uncached / write-combine)
    mem_props.add_memory_type(
        MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
            | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT
            | MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT,
    );

    // Type 2: Host-visible & host-cached & host-coherent (if GPU has ACE/ACE-Lite coherency)
    let is_coherent = gpu_info.selected_coherency != DRM_PANTHOR_GPU_COHERENCY_NONE;
    if is_coherent {
        mem_props.add_memory_type(
            MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT
                | MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT,
        );
    }

    // Type 3: Host-visible & host-cached non-coherent (write-back cached with explicit flush/invalidate)
    mem_props.add_memory_type(
        MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
            | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT
            | MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT,
    );

    mem_props.increment_heap_count();
    mem_props
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PanthorPhysicalDevice {
    descriptor: OwnedDescriptor,
    gpu_info: drm_panthor_gpu_info,
    csif_info: drm_panthor_csif_info,
}

#[allow(dead_code)]
impl PanthorPhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> Self {
        let gpu_info = panthor_query_gpu_info(descriptor.as_fd()).unwrap_or_default();
        let csif_info = panthor_query_csif_info(descriptor.as_fd()).unwrap_or_default();
        Self {
            descriptor,
            gpu_info,
            csif_info,
        }
    }

    pub fn gpu_info(&self) -> &drm_panthor_gpu_info {
        &self.gpu_info
    }

    pub fn csif_info(&self) -> &drm_panthor_csif_info {
        &self.csif_info
    }

    pub fn gpu_id(&self) -> u32 {
        self.gpu_info.gpu_id
    }

    pub fn device_id(&self) -> u16 {
        (self.gpu_info.gpu_id >> 12) as u16
    }
}

impl PlatformPhysicalDevice for PanthorPhysicalDevice {
    fn as_descriptor(&self) -> Option<&OwnedDescriptor> {
        Some(&self.descriptor)
    }
}

impl AsVirtGpu for PanthorPhysicalDevice {}
impl PhysicalDevice for PanthorPhysicalDevice {}
unsafe impl Send for PanthorPhysicalDevice {}
unsafe impl Sync for PanthorPhysicalDevice {}

impl GenericPhysicalDevice for PanthorPhysicalDevice {
    fn create_device(
        self: Arc<Self>,
        _device_info: &MagmaPhysicalDeviceInfo,
    ) -> MagmaGpuResult<Arc<dyn Device>> {
        Ok(Arc::new(Panthor::new(self)?))
    }

    fn query_memory_properties(&self) -> MagmaGpuResult<MagmaMemoryProperties> {
        Ok(panthor_query_memory_properties(&self.gpu_info))
    }

    fn query_queue_family_properties(&self) -> MagmaGpuResult<Vec<MagmaQueueFamilyProperties>> {
        Ok(vec![
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Graphics
                    | MagmaQueueFlags::Compute
                    | MagmaQueueFlags::Transfer
                    | MagmaQueueFlags::SparseBinding,
                2,
            ),
            MagmaQueueFamilyProperties::new(MagmaQueueFlags::SparseBinding, 1),
        ])
    }
}

pub struct Panthor {
    physical_device: Arc<PanthorPhysicalDevice>,
    mem_props: MagmaMemoryProperties,
}

impl Panthor {
    pub fn new(physical_device: Arc<PanthorPhysicalDevice>) -> MagmaGpuResult<Panthor> {
        let mem_props = physical_device.query_memory_properties()?;
        Ok(Panthor {
            physical_device,
            mem_props,
        })
    }
}

impl PlatformDevice for Panthor {}
impl Device for Panthor {}
unsafe impl Send for Panthor {}
unsafe impl Sync for Panthor {}

impl GenericDevice for Panthor {
    fn get_memory_budget(&self, _heap_idx: u32) -> MagmaGpuResult<MagmaHeapBudget> {
        Err(MagmaGpuError::Unsupported)
    }

    fn create_address_space(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn AddressSpace>> {
        let mut vm_create = drm_panthor_vm_create {
            flags: 0,
            user_va_range: 0,
            ..Default::default()
        };

        unsafe {
            drm_ioctl_panthor_vm_create(self.physical_device.as_fd().unwrap(), &mut vm_create)?;
        }

        Ok(Arc::new(PanthorAddressSpace {
            physical_device: self.physical_device.clone(),
            vm_id: vm_create.id,
        }))
    }

    fn create_queue(
        self: Arc<Self>,
        address_space: &Arc<dyn AddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> MagmaGpuResult<Arc<dyn Queue>> {
        let vm_id = address_space.as_vm_id().ok_or(MagmaGpuError::Unsupported)?;
        let is_bind_queue =
            (info.flags & MagmaQueueFlags::SparseBinding.bits()) != 0 || info.queue_family_idx == 1;

        if is_bind_queue {
            return Ok(Arc::new(PanthorQueue {
                physical_device: self.physical_device.clone(),
                group_handle: 0,
                vm_id,
                is_bind_queue: true,
            }));
        }

        let gpu_info = self.physical_device.gpu_info();
        let compute_core_mask = if gpu_info.shader_present != 0 {
            gpu_info.shader_present
        } else {
            1
        };
        let fragment_core_mask = if gpu_info.shader_present != 0 {
            gpu_info.shader_present
        } else {
            1
        };
        let tiler_core_mask = if gpu_info.tiler_present != 0 {
            gpu_info.tiler_present
        } else {
            1
        };

        let max_compute_cores = compute_core_mask.count_ones().max(1) as u8;
        let max_fragment_cores = fragment_core_mask.count_ones().max(1) as u8;
        let max_tiler_cores = tiler_core_mask.count_ones().max(1) as u8;

        let priority = match info.priority {
            0 => PANTHOR_GROUP_PRIORITY_LOW as u8,
            1 => PANTHOR_GROUP_PRIORITY_MEDIUM as u8,
            2 => PANTHOR_GROUP_PRIORITY_HIGH as u8,
            _ => PANTHOR_GROUP_PRIORITY_MEDIUM as u8,
        };

        let qc = [drm_panthor_queue_create {
            priority: 1,
            pad: [0; 3],
            ringbuf_size: 64 * 1024,
        }];

        let mut gc = drm_panthor_group_create {
            queues: drm_panthor_obj_array {
                stride: std::mem::size_of::<drm_panthor_queue_create>() as u32,
                count: qc.len() as u32,
                array: qc.as_ptr() as u64,
            },
            max_compute_cores,
            max_fragment_cores,
            max_tiler_cores,
            priority,
            pad: 0,
            compute_core_mask,
            fragment_core_mask,
            tiler_core_mask,
            vm_id,
            group_handle: 0,
        };

        unsafe {
            drm_ioctl_panthor_group_create(self.physical_device.as_fd().unwrap(), &mut gc)?;
        }

        Ok(Arc::new(PanthorQueue {
            physical_device: self.physical_device.clone(),
            group_handle: gc.group_handle,
            vm_id,
            is_bind_queue: false,
        }))
    }

    fn create_buffer(
        self: Arc<Self>,
        create_info: &MagmaCreateBufferInfo,
    ) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let buf = PanthorBuffer::new(self.physical_device.clone(), create_info, &self.mem_props)?;
        Ok(Arc::new(buf))
    }

    fn import(self: Arc<Self>, info: MagmaImportHandleInfo) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let gem_handle = self.physical_device.import(info.handle)?;
        let buf = PanthorBuffer::from_existing(
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

struct PanthorAddressSpace {
    physical_device: Arc<PanthorPhysicalDevice>,
    vm_id: u32,
}

impl Drop for PanthorAddressSpace {
    fn drop(&mut self) {
        let destroy = drm_panthor_vm_destroy {
            id: self.vm_id,
            pad: 0,
        };
        let result = unsafe {
            drm_ioctl_panthor_vm_destroy(self.physical_device.as_fd().unwrap(), &destroy)
        };
        log_status!(result);
    }
}

impl GenericAddressSpace for PanthorAddressSpace {
    fn as_vm_id(&self) -> Option<u32> {
        Some(self.vm_id)
    }

    fn map_buffer_gpu(
        &self,
        buffer: &Arc<dyn Buffer>,
        buffer_offset: u64,
        gpu_va: u64,
        size: u64,
        flags: MagmaGpuMapFlags,
    ) -> MagmaGpuResult<()> {
        let gem_handle = buffer.as_gem_handle().ok_or(MagmaGpuError::Unsupported)?;
        let mut bind_flags = DRM_PANTHOR_VM_BIND_OP_TYPE_MAP as u32;

        if (flags & MagmaGpuMapFlags::Execute).bits() == 0 {
            bind_flags |= DRM_PANTHOR_VM_BIND_OP_MAP_NOEXEC as u32;
        }
        if (flags & MagmaGpuMapFlags::Write).bits() == 0 {
            bind_flags |= DRM_PANTHOR_VM_BIND_OP_MAP_READONLY as u32;
        }

        let op = drm_panthor_vm_bind_op {
            flags: bind_flags,
            bo_handle: gem_handle,
            bo_offset: buffer_offset,
            va: gpu_va,
            size,
            syncs: drm_panthor_obj_array {
                stride: 0,
                count: 0,
                array: 0,
            },
        };

        let mut vm_bind = drm_panthor_vm_bind {
            vm_id: self.vm_id,
            flags: 0,
            ops: drm_panthor_obj_array {
                stride: std::mem::size_of::<drm_panthor_vm_bind_op>() as u32,
                count: 1,
                array: &op as *const _ as u64,
            },
        };

        unsafe {
            drm_ioctl_panthor_vm_bind(self.physical_device.as_fd().unwrap(), &mut vm_bind)?;
        }
        Ok(())
    }

    fn unmap_buffer_gpu(&self, gpu_va: u64, size: u64) -> MagmaGpuResult<()> {
        let op = drm_panthor_vm_bind_op {
            flags: DRM_PANTHOR_VM_BIND_OP_TYPE_UNMAP as u32,
            bo_handle: 0,
            bo_offset: 0,
            va: gpu_va,
            size,
            syncs: drm_panthor_obj_array {
                stride: 0,
                count: 0,
                array: 0,
            },
        };

        let mut vm_bind = drm_panthor_vm_bind {
            vm_id: self.vm_id,
            flags: 0,
            ops: drm_panthor_obj_array {
                stride: std::mem::size_of::<drm_panthor_vm_bind_op>() as u32,
                count: 1,
                array: &op as *const _ as u64,
            },
        };

        unsafe {
            drm_ioctl_panthor_vm_bind(self.physical_device.as_fd().unwrap(), &mut vm_bind)?;
        }
        Ok(())
    }
}

impl AddressSpace for PanthorAddressSpace {}
unsafe impl Send for PanthorAddressSpace {}
unsafe impl Sync for PanthorAddressSpace {}

struct PanthorQueue {
    physical_device: Arc<PanthorPhysicalDevice>,
    group_handle: u32,
    vm_id: u32,
    is_bind_queue: bool,
}

impl Drop for PanthorQueue {
    fn drop(&mut self) {
        if !self.is_bind_queue && self.group_handle != 0 {
            let destroy = drm_panthor_group_destroy {
                group_handle: self.group_handle,
                pad: 0,
            };
            let result = unsafe {
                drm_ioctl_panthor_group_destroy(self.physical_device.as_fd().unwrap(), &destroy)
            };
            log_status!(result);
        }
    }
}

impl GenericQueue for PanthorQueue {
    fn submit_command(&self, submit_info: &MagmaSubmitInfo) -> MagmaGpuResult<()> {
        let fd = self.physical_device.as_fd().unwrap();
        let mut sync_ops: Vec<drm_panthor_sync_op> = Vec::new();

        if let Some(sync_info) = submit_info.sync_info() {
            for &handle in sync_info
                .wait_sync_objs
                .iter()
                .take(sync_info.num_wait_sync_objs as usize)
            {
                if handle != 0 {
                    sync_ops.push(drm_panthor_sync_op {
                        flags: (DRM_PANTHOR_SYNC_OP_HANDLE_TYPE_SYNCOBJ as u32)
                            | (DRM_PANTHOR_SYNC_OP_WAIT as u32),
                        handle,
                        timeline_value: 0,
                    });
                }
            }

            for &handle in sync_info
                .signal_sync_objs
                .iter()
                .take(sync_info.num_signal_sync_objs as usize)
            {
                if handle != 0 {
                    sync_ops.push(drm_panthor_sync_op {
                        flags: (DRM_PANTHOR_SYNC_OP_HANDLE_TYPE_SYNCOBJ as u32)
                            | (DRM_PANTHOR_SYNC_OP_SIGNAL as u32),
                        handle,
                        timeline_value: 0,
                    });
                }
            }
        }

        if self.is_bind_queue {
            let op = drm_panthor_vm_bind_op {
                flags: DRM_PANTHOR_VM_BIND_OP_TYPE_SYNC_ONLY as u32,
                bo_handle: 0,
                bo_offset: 0,
                va: 0,
                size: 0,
                syncs: drm_panthor_obj_array {
                    stride: std::mem::size_of::<drm_panthor_sync_op>() as u32,
                    count: sync_ops.len() as u32,
                    array: sync_ops.as_ptr() as u64,
                },
            };

            let mut vm_bind = drm_panthor_vm_bind {
                vm_id: self.vm_id,
                flags: DRM_PANTHOR_VM_BIND_ASYNC,
                ops: drm_panthor_obj_array {
                    stride: std::mem::size_of::<drm_panthor_vm_bind_op>() as u32,
                    count: 1,
                    array: &op as *const _ as u64,
                },
            };

            unsafe {
                drm_ioctl_panthor_vm_bind(fd, &mut vm_bind)?;
            }
        } else {
            let (stream_addr, stream_size) = if let Some(as_info) = submit_info.address_space_info()
            {
                (as_info.command_va, as_info.length as u32)
            } else if let Some(buf_info) = submit_info.buffer_info() {
                (buf_info.start_offset, buf_info.length as u32)
            } else {
                (0, 0)
            };

            let qsubmit = drm_panthor_queue_submit {
                queue_index: 0,
                stream_size,
                stream_addr,
                latest_flush: 0,
                pad: 0,
                syncs: drm_panthor_obj_array {
                    stride: std::mem::size_of::<drm_panthor_sync_op>() as u32,
                    count: sync_ops.len() as u32,
                    array: sync_ops.as_ptr() as u64,
                },
            };

            let mut gsubmit = drm_panthor_group_submit {
                group_handle: self.group_handle,
                pad: 0,
                queue_submits: drm_panthor_obj_array {
                    stride: std::mem::size_of::<drm_panthor_queue_submit>() as u32,
                    count: 1,
                    array: &qsubmit as *const _ as u64,
                },
            };

            unsafe {
                drm_ioctl_panthor_group_submit(fd, &mut gsubmit)?;
            }
        }
        Ok(())
    }

    fn check_status(&self) -> MagmaResult<()> {
        if self.is_bind_queue {
            let mut state = drm_panthor_vm_get_state {
                vm_id: self.vm_id,
                state: 0,
            };
            let fd = self
                .physical_device
                .as_fd()
                .ok_or(MagmaError::InternalError)?;
            unsafe {
                drm_ioctl_panthor_vm_get_state(fd, &mut state)
                    .map_err(|_| MagmaError::InternalError)?;
            }
            if state.state == DRM_PANTHOR_VM_STATE_UNUSABLE {
                return Err(MagmaError::ContextKilled);
            }
            return Ok(());
        }

        let mut state = drm_panthor_group_get_state {
            group_handle: self.group_handle,
            state: 0,
            fatal_queues: 0,
            pad: 0,
        };
        let fd = self
            .physical_device
            .as_fd()
            .ok_or(MagmaError::InternalError)?;
        unsafe {
            drm_ioctl_panthor_group_get_state(fd, &mut state)
                .map_err(|_| MagmaError::InternalError)?;
        }
        if (state.state & DRM_PANTHOR_GROUP_STATE_TIMEDOUT) != 0 {
            return Err(MagmaError::TimedOut);
        }
        if (state.state & DRM_PANTHOR_GROUP_STATE_FATAL_FAULT) != 0 {
            return Err(MagmaError::ContextKilled);
        }
        Ok(())
    }
}

impl Queue for PanthorQueue {}
unsafe impl Send for PanthorQueue {}
unsafe impl Sync for PanthorQueue {}

struct PanthorBuffer {
    physical_device: Arc<PanthorPhysicalDevice>,
    gem_handle: u32,
    size: usize,
}

impl PanthorBuffer {
    fn new(
        physical_device: Arc<PanthorPhysicalDevice>,
        create_info: &MagmaCreateBufferInfo,
        mem_props: &MagmaMemoryProperties,
    ) -> MagmaGpuResult<PanthorBuffer> {
        let mem_type = mem_props.get_memory_type(create_info.memory_type_idx);
        let mut flags = 0u32;
        if mem_type.is_cached() {
            flags |= DRM_PANTHOR_BO_WB_MMAP;
        }

        let mut bo_create = drm_panthor_bo_create {
            size: create_info.size,
            flags,
            exclusive_vm_id: 0,
            ..Default::default()
        };

        unsafe {
            drm_ioctl_panthor_bo_create(physical_device.as_fd().unwrap(), &mut bo_create)?;
        }

        Ok(PanthorBuffer {
            physical_device,
            gem_handle: bo_create.handle,
            size: bo_create.size as usize,
        })
    }

    fn from_existing(
        physical_device: Arc<PanthorPhysicalDevice>,
        gem_handle: u32,
        size: usize,
    ) -> MagmaGpuResult<PanthorBuffer> {
        Ok(PanthorBuffer {
            physical_device,
            gem_handle,
            size,
        })
    }
}

impl Drop for PanthorBuffer {
    fn drop(&mut self) {
        self.physical_device.close(self.gem_handle);
    }
}

impl GenericBuffer for PanthorBuffer {
    fn map(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn MappedRegion>> {
        let mut mmap_offset = drm_panthor_bo_mmap_offset {
            handle: self.gem_handle,
            ..Default::default()
        };

        let offset = unsafe {
            drm_ioctl_panthor_bo_mmap_offset(
                self.physical_device.as_fd().unwrap(),
                &mut mmap_offset,
            )?;
            mmap_offset.offset
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
        ranges: &[MagmaMappedMemoryRange],
    ) -> MagmaGpuResult<()> {
        let fd = self.physical_device.as_fd().unwrap();
        if ranges.is_empty() {
            let sync_op = drm_panthor_bo_sync_op {
                handle: self.gem_handle,
                type_: DRM_PANTHOR_BO_SYNC_CPU_CACHE_FLUSH_AND_INVALIDATE,
                offset: 0,
                size: self.size as u64,
            };
            let mut sync = drm_panthor_bo_sync {
                ops: drm_panthor_obj_array {
                    stride: std::mem::size_of::<drm_panthor_bo_sync_op>() as u32,
                    count: 1,
                    array: &sync_op as *const _ as u64,
                },
            };
            unsafe {
                drm_ioctl_panthor_bo_sync(fd, &mut sync)?;
            }
        } else {
            let sync_ops: Vec<drm_panthor_bo_sync_op> = ranges
                .iter()
                .map(|r| drm_panthor_bo_sync_op {
                    handle: self.gem_handle,
                    type_: DRM_PANTHOR_BO_SYNC_CPU_CACHE_FLUSH_AND_INVALIDATE,
                    offset: r.offset,
                    size: r.size,
                })
                .collect();
            let mut sync = drm_panthor_bo_sync {
                ops: drm_panthor_obj_array {
                    stride: std::mem::size_of::<drm_panthor_bo_sync_op>() as u32,
                    count: sync_ops.len() as u32,
                    array: sync_ops.as_ptr() as u64,
                },
            };
            unsafe {
                drm_ioctl_panthor_bo_sync(fd, &mut sync)?;
            }
        }
        Ok(())
    }

    fn flush(&self, _sync_flags: u64, ranges: &[MagmaMappedMemoryRange]) -> MagmaGpuResult<()> {
        let fd = self.physical_device.as_fd().unwrap();
        if ranges.is_empty() {
            let sync_op = drm_panthor_bo_sync_op {
                handle: self.gem_handle,
                type_: DRM_PANTHOR_BO_SYNC_CPU_CACHE_FLUSH,
                offset: 0,
                size: self.size as u64,
            };
            let mut sync = drm_panthor_bo_sync {
                ops: drm_panthor_obj_array {
                    stride: std::mem::size_of::<drm_panthor_bo_sync_op>() as u32,
                    count: 1,
                    array: &sync_op as *const _ as u64,
                },
            };
            unsafe {
                drm_ioctl_panthor_bo_sync(fd, &mut sync)?;
            }
        } else {
            let sync_ops: Vec<drm_panthor_bo_sync_op> = ranges
                .iter()
                .map(|r| drm_panthor_bo_sync_op {
                    handle: self.gem_handle,
                    type_: DRM_PANTHOR_BO_SYNC_CPU_CACHE_FLUSH,
                    offset: r.offset,
                    size: r.size,
                })
                .collect();
            let mut sync = drm_panthor_bo_sync {
                ops: drm_panthor_obj_array {
                    stride: std::mem::size_of::<drm_panthor_bo_sync_op>() as u32,
                    count: sync_ops.len() as u32,
                    array: sync_ops.as_ptr() as u64,
                },
            };
            unsafe {
                drm_ioctl_panthor_bo_sync(fd, &mut sync)?;
            }
        }
        Ok(())
    }

    fn as_gem_handle(&self) -> Option<u32> {
        Some(self.gem_handle)
    }
}

impl Buffer for PanthorBuffer {}
unsafe impl Send for PanthorBuffer {}
unsafe impl Sync for PanthorBuffer {}
