// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
use std::sync::Arc;

use crate::ioctl_readwrite;
use crate::ioctl_write_ptr;
use crate::sys::linux::drm::DrmSyncObject;

use magma_gpu::util::Error as MagmaGpuError;
use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MappedRegion;
use magma_gpu::util::OwnedDescriptor;
use magma_gpu::util::Result as MagmaGpuResult;

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
use crate::magma_defines::MagmaQueueFlags;
use crate::magma_defines::MagmaSubmitInfo;
use crate::magma_defines::MagmaSyncType;

use crate::sys::linux::bindings::drm_bindings::DRM_COMMAND_BASE;
use crate::sys::linux::bindings::drm_bindings::DRM_IOCTL_BASE;
use crate::sys::linux::bindings::msm_bindings::*;
use crate::sys::linux::PlatformDevice;
use crate::sys::linux::PlatformPhysicalDevice;

ioctl_readwrite!(
    drm_ioctl_msm_gem_new,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_MSM_GEM_NEW,
    drm_msm_gem_new
);

ioctl_readwrite!(
    drm_ioctl_msm_gem_info,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_MSM_GEM_INFO,
    drm_msm_gem_info
);

ioctl_write_ptr!(
    msm_gem_cpu_prep,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_MSM_GEM_CPU_PREP,
    drm_msm_gem_cpu_prep
);

ioctl_write_ptr!(
    msm_gem_cpu_fini,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_MSM_GEM_CPU_FINI,
    drm_msm_gem_cpu_fini
);

ioctl_readwrite!(
    msm_submitqueue_new,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_MSM_SUBMITQUEUE_NEW,
    drm_msm_submitqueue
);

ioctl_write_ptr!(
    msm_submitqueue_close,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_MSM_SUBMITQUEUE_CLOSE,
    __u32
);

ioctl_readwrite!(
    msm_gem_submit,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_MSM_GEM_SUBMIT,
    drm_msm_gem_submit
);

struct MsmAddressSpace {
    _physical_device: Arc<dyn PhysicalDevice>,
}

impl GenericAddressSpace for MsmAddressSpace {}
impl AddressSpace for MsmAddressSpace {}

struct MsmQueue {
    physical_device: Arc<dyn PhysicalDevice>,
    submit_queue_id: u32,
}

impl Drop for MsmQueue {
    fn drop(&mut self) {
        // SAFETY: This is a valid file descriptor and a valid submitqueue id.
        unsafe {
            let _ =
                msm_submitqueue_close(self.physical_device.as_fd().unwrap(), &self.submit_queue_id);
        }
    }
}

impl GenericQueue for MsmQueue {
    fn submit_command(&self, submit_info: &MagmaSubmitInfo) -> MagmaGpuResult<()> {
        let buf_info = submit_info
            .buffer_info()
            .ok_or(MagmaGpuError::Unsupported)?;

        let bo = drm_msm_gem_submit_bo {
            flags: MSM_SUBMIT_BO_READ,
            handle: buf_info.command_buffer,
            presumed: 0,
        };

        let cmd = drm_msm_gem_submit_cmd {
            type_: MSM_SUBMIT_CMD_BUF,
            submit_idx: 0,
            submit_offset: buf_info.start_offset as u32,
            size: buf_info.length as u32,
            pad: 0,
            nr_relocs: 0,
            __bindgen_anon_1: drm_msm_gem_submit_cmd__bindgen_ty_1 { relocs: 0 },
        };

        let mut in_syncs: Vec<drm_msm_syncobj> = Vec::new();
        let mut out_syncs: Vec<drm_msm_syncobj> = Vec::new();
        let mut submit_flags = 0u32;

        if let Some(sync_info) = submit_info.sync_info() {
            for &handle in sync_info
                .wait_sync_objs
                .iter()
                .take(sync_info.num_wait_sync_objs as usize)
            {
                if handle != 0 {
                    in_syncs.push(drm_msm_syncobj {
                        handle,
                        flags: 0,
                        point: 0,
                    });
                }
            }
            for &handle in sync_info
                .signal_sync_objs
                .iter()
                .take(sync_info.num_signal_sync_objs as usize)
            {
                if handle != 0 {
                    out_syncs.push(drm_msm_syncobj {
                        handle,
                        flags: 0,
                        point: 0,
                    });
                }
            }
        }

        if !in_syncs.is_empty() {
            submit_flags |= MSM_SUBMIT_SYNCOBJ_IN;
        }
        if !out_syncs.is_empty() {
            submit_flags |= MSM_SUBMIT_SYNCOBJ_OUT;
        }

        let mut submit = drm_msm_gem_submit {
            flags: submit_flags,
            fence: 0,
            nr_bos: 1,
            nr_cmds: 1,
            bos: &bo as *const _ as u64,
            cmds: &cmd as *const _ as u64,
            fence_fd: -1,
            queueid: self.submit_queue_id,
            in_syncobjs: in_syncs.as_ptr() as u64,
            out_syncobjs: out_syncs.as_ptr() as u64,
            nr_in_syncobjs: in_syncs.len() as u32,
            nr_out_syncobjs: out_syncs.len() as u32,
            syncobj_stride: std::mem::size_of::<drm_msm_syncobj>() as u32,
            pad: 0,
        };

        unsafe {
            msm_gem_submit(self.physical_device.as_fd().unwrap(), &mut submit)?;
        }
        Ok(())
    }
}

impl Queue for MsmQueue {}

#[derive(Debug)]
pub struct MsmPhysicalDevice {
    descriptor: OwnedDescriptor,
}

impl MsmPhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> Self {
        Self { descriptor }
    }
}

impl PlatformPhysicalDevice for MsmPhysicalDevice {
    fn as_descriptor(&self) -> Option<&OwnedDescriptor> {
        Some(&self.descriptor)
    }
}

impl AsVirtGpu for MsmPhysicalDevice {}
impl PhysicalDevice for MsmPhysicalDevice {}
unsafe impl Send for MsmPhysicalDevice {}
unsafe impl Sync for MsmPhysicalDevice {}

impl GenericPhysicalDevice for MsmPhysicalDevice {
    fn create_device(
        self: Arc<Self>,
        _info: &MagmaPhysicalDeviceInfo,
    ) -> MagmaGpuResult<Arc<dyn Device>> {
        Ok(Arc::new(Msm::new(self)))
    }

    fn query_memory_properties(&self) -> MagmaGpuResult<MagmaMemoryProperties> {
        Ok(MagmaMemoryProperties::default())
    }

    fn query_queue_family_properties(&self) -> MagmaGpuResult<Vec<MagmaQueueFamilyProperties>> {
        Ok(vec![MagmaQueueFamilyProperties::new(
            MagmaQueueFlags::Graphics | MagmaQueueFlags::Compute,
            1,
        )])
    }
}

pub struct Msm {
    physical_device: Arc<dyn PhysicalDevice>,
    mem_props: MagmaMemoryProperties,
}

struct MsmBuffer {
    physical_device: Arc<dyn PhysicalDevice>,
    gem_handle: u32,
    size: usize,
}

impl Msm {
    pub fn new(physical_device: Arc<dyn PhysicalDevice>) -> Msm {
        Msm {
            physical_device,
            mem_props: Default::default(),
        }
    }
}

impl GenericDevice for Msm {
    fn get_memory_budget(&self, _heap_idx: u32) -> MagmaGpuResult<MagmaHeapBudget> {
        Err(MagmaGpuError::Unsupported)
    }

    fn create_address_space(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn AddressSpace>> {
        Ok(Arc::new(MsmAddressSpace {
            _physical_device: self.physical_device.clone(),
        }))
    }

    fn create_queue(
        self: Arc<Self>,
        _address_space: &Arc<dyn AddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> MagmaGpuResult<Arc<dyn Queue>> {
        let mut new_submit_queue = drm_msm_submitqueue {
            flags: 0,
            prio: info.priority,
            ..Default::default()
        };

        // SAFETY: This is a valid file descriptor.
        unsafe {
            msm_submitqueue_new(self.physical_device.as_fd().unwrap(), &mut new_submit_queue)?;
        }

        Ok(Arc::new(MsmQueue {
            physical_device: self.physical_device.clone(),
            submit_queue_id: new_submit_queue.id,
        }))
    }

    fn create_buffer(
        self: Arc<Self>,
        create_info: &MagmaCreateBufferInfo,
    ) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let buf = MsmBuffer::new(self.physical_device.clone(), create_info, &self.mem_props)?;
        Ok(Arc::new(buf))
    }

    fn import(self: Arc<Self>, info: MagmaImportHandleInfo) -> MagmaGpuResult<Arc<dyn Buffer>> {
        let gem_handle = self.physical_device.import(info.handle)?;
        let buf = MsmBuffer::from_existing(
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

impl PlatformDevice for Msm {}
impl Device for Msm {}

impl MsmBuffer {
    fn new(
        physical_device: Arc<dyn PhysicalDevice>,
        create_info: &MagmaCreateBufferInfo,
        _mem_props: &MagmaMemoryProperties,
    ) -> MagmaGpuResult<MsmBuffer> {
        let mut gem_new = drm_msm_gem_new {
            size: create_info.size,
            flags: 0,
            ..Default::default()
        };

        // SAFETY: This is a well-formed ioctl conforming the driver specificiation.
        unsafe {
            drm_ioctl_msm_gem_new(physical_device.as_fd().unwrap(), &mut gem_new)?;
        }

        Ok(MsmBuffer {
            physical_device,
            gem_handle: gem_new.handle,
            size: create_info.size.try_into()?,
        })
    }

    fn from_existing(
        physical_device: Arc<dyn PhysicalDevice>,
        gem_handle: u32,
        size: usize,
    ) -> MagmaGpuResult<MsmBuffer> {
        Ok(MsmBuffer {
            physical_device,
            gem_handle,
            size,
        })
    }
}

impl GenericBuffer for MsmBuffer {
    fn map(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn MappedRegion>> {
        let mut gem_info: drm_msm_gem_info = drm_msm_gem_info {
            handle: self.gem_handle,
            info: MSM_INFO_GET_OFFSET,
            ..Default::default()
        };

        // SAFETY:
        // Valid arguments are supplied for the following arguments:
        //   - Underlying descriptor
        //   - drm_msm_gem_info
        let offset = unsafe {
            drm_ioctl_msm_gem_info(self.physical_device.as_fd().unwrap(), &mut gem_info)?;
            gem_info.value
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
        let prep = drm_msm_gem_cpu_prep {
            handle: self.gem_handle,
            op: MSM_PREP_READ | MSM_PREP_WRITE,
            ..Default::default()
        };

        // SAFETY: This is a valid file descriptor and a valid gem handle.
        unsafe {
            msm_gem_cpu_prep(self.physical_device.as_fd().unwrap(), &prep)?;
        }
        Ok(())
    }

    fn flush(&self, _sync_flags: u64, _ranges: &[MagmaMappedMemoryRange]) -> MagmaGpuResult<()> {
        let fini = drm_msm_gem_cpu_fini {
            handle: self.gem_handle,
        };

        // SAFETY: This is a valid file descriptor and a valid gem handle.
        unsafe {
            msm_gem_cpu_fini(self.physical_device.as_fd().unwrap(), &fini)?;
        }
        Ok(())
    }

    fn as_gem_handle(&self) -> Option<u32> {
        Some(self.gem_handle)
    }
}

impl Drop for MsmBuffer {
    fn drop(&mut self) {
        // GEM close
    }
}

impl Buffer for MsmBuffer {}

unsafe impl Send for Msm {}
unsafe impl Sync for Msm {}

unsafe impl Send for MsmAddressSpace {}
unsafe impl Sync for MsmAddressSpace {}

unsafe impl Send for MsmQueue {}
unsafe impl Sync for MsmQueue {}

unsafe impl Send for MsmBuffer {}
unsafe impl Sync for MsmBuffer {}
