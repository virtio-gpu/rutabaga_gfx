// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

//! Magma: Rust implementation of Fuchsia's driver model.
//!
//! Design found at <https://fuchsia.dev/fuchsia-third_party/magma_gpu/src/development/graphics/magma/concepts/design>.

use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MappedRegion;

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
use crate::magma_defines::MagmaStructureType;
use crate::magma_defines::MagmaStructureTypeHeader;
use crate::magma_defines::MagmaSubmitAddressSpaceInfo;
use crate::magma_defines::MagmaSubmitBufferInfo;
use crate::magma_defines::MagmaSubmitInfo;
use crate::magma_defines::MagmaSubmitSyncInfo;
use crate::magma_defines::MagmaSyncType;
use crate::protocol::MagmaGpuMapFlags;
use crate::protocol::MAGMA_MAX_SYNCOBJS;

use crate::traits::AddressSpace;
use crate::traits::Buffer;
use crate::traits::Device;
use crate::traits::PhysicalDevice;
use crate::traits::Queue;
use crate::traits::SyncObject;

use crate::magma_kumquat::enumerate_devices as magma_kumquat_enumerate_devices;
use crate::sys::platform::enumerate_devices as platform_enumerate_devices;

const VIRTGPU_KUMQUAT_ENABLED: &str = "VIRTGPU_KUMQUAT";

#[repr(C)]
#[derive(Clone)]
pub struct MagmaPhysicalDevice {
    physical_device: Arc<dyn PhysicalDevice>,
    info: MagmaPhysicalDeviceInfo,
}

#[derive(Clone)]
pub struct MagmaDevice {
    device: Arc<dyn Device>,
}

#[derive(Clone)]
pub struct MagmaAddressSpace {
    address_space: Arc<dyn AddressSpace>,
}

#[derive(Clone)]
pub struct MagmaQueue {
    queue: Arc<dyn Queue>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagmaMapping {
    pub ptr: *mut c_void,
    pub size: u64,
}

unsafe impl Send for MagmaMapping {}
unsafe impl Sync for MagmaMapping {}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MagmaHandle {
    pub os_handle: i64,
    pub handle_type: u32,
}

#[derive(Clone)]
pub struct MagmaBuffer {
    buffer: Arc<dyn Buffer>,
    mapping: Arc<Mutex<Option<Arc<dyn MappedRegion>>>>,
}

#[derive(Clone)]
pub struct MagmaSyncObj {
    pub(crate) sync_obj: Arc<dyn SyncObject>,
}

impl MagmaSyncObj {
    pub fn wait(&self, timeout_ns: u64) -> MagmaResult<()> {
        self.sync_obj.wait(timeout_ns)
    }

    pub fn signal(&self) -> MagmaResult<()> {
        self.sync_obj.signal()?;
        Ok(())
    }

    pub fn timeline_wait(&self, point: u64, timeout_ns: u64, flags: u32) -> MagmaResult<()> {
        self.sync_obj.timeline_wait(point, timeout_ns, flags)
    }

    pub fn timeline_signal(&self, point: u64) -> MagmaResult<()> {
        self.sync_obj.timeline_signal(point)?;
        Ok(())
    }

    pub fn timeline_query(&self) -> MagmaResult<u64> {
        let point = self.sync_obj.timeline_query()?;
        Ok(point)
    }

    pub fn export_fence(&self) -> MagmaResult<MagmaGpuHandle> {
        let handle = self.sync_obj.export_fence()?;
        Ok(handle)
    }

    pub fn import(&self, handle: MagmaGpuHandle) -> MagmaResult<()> {
        self.sync_obj.import(handle)?;
        Ok(())
    }

    pub fn as_raw_handle(&self) -> Option<u32> {
        self.sync_obj.as_raw_handle()
    }

    pub fn get_type(&self) -> MagmaSyncType {
        self.sync_obj.get_type()
    }

    pub fn is_timeline(&self) -> bool {
        self.sync_obj.is_timeline()
    }
}

pub fn magma_enumerate_devices() -> MagmaResult<Vec<MagmaPhysicalDevice>> {
    let devices = match std::env::var(VIRTGPU_KUMQUAT_ENABLED) {
        Ok(_) => magma_kumquat_enumerate_devices()?,
        Err(_) => platform_enumerate_devices()?,
    };

    Ok(devices)
}

impl MagmaPhysicalDevice {
    pub(crate) fn new(
        physical_device: Arc<dyn PhysicalDevice>,
        info: MagmaPhysicalDeviceInfo,
    ) -> MagmaPhysicalDevice {
        MagmaPhysicalDevice {
            physical_device,
            info,
        }
    }

    pub fn info(&self) -> &MagmaPhysicalDeviceInfo {
        &self.info
    }

    pub fn get_memory_properties(&self) -> MagmaResult<MagmaMemoryProperties> {
        Ok(self.info.memory_properties)
    }

    pub fn get_queue_family_properties(&self) -> MagmaResult<Vec<MagmaQueueFamilyProperties>> {
        let count = (self.info.queue_family_count as usize).min(self.info.queue_families.len());
        Ok(self.info.queue_families[..count].to_vec())
    }

    pub fn create_device(&self) -> MagmaResult<MagmaDevice> {
        let device = self.physical_device.clone().create_device(&self.info)?;
        Ok(MagmaDevice { device })
    }
}

impl MagmaDevice {
    pub fn get_memory_budget(&self, heap_idx: u32) -> MagmaResult<MagmaHeapBudget> {
        let budget = self.device.get_memory_budget(heap_idx)?;
        Ok(budget)
    }

    pub fn create_address_space(&self) -> MagmaResult<MagmaAddressSpace> {
        let address_space = self.device.clone().create_address_space()?;
        Ok(MagmaAddressSpace { address_space })
    }

    pub fn create_queue(
        &self,
        address_space: &MagmaAddressSpace,
        info: &MagmaCreateQueueInfo,
    ) -> MagmaResult<MagmaQueue> {
        let queue = self
            .device
            .clone()
            .create_queue(&address_space.address_space, info)?;
        Ok(MagmaQueue { queue })
    }

    pub fn create_buffer(&self, create_info: &MagmaCreateBufferInfo) -> MagmaResult<MagmaBuffer> {
        let buffer = self.device.clone().create_buffer(create_info)?;
        Ok(MagmaBuffer {
            buffer,
            mapping: Arc::new(Mutex::new(None)),
        })
    }

    // FIXME: we probably want to import with a memory type
    pub fn import(&self, info: MagmaImportHandleInfo) -> MagmaResult<MagmaBuffer> {
        let buffer = self.device.clone().import(info)?;
        Ok(MagmaBuffer {
            buffer,
            mapping: Arc::new(Mutex::new(None)),
        })
    }

    pub fn create_sync_obj(&self, info: &MagmaCreateSyncObjInfo) -> MagmaResult<MagmaSyncObj> {
        let sync_obj = self.device.clone().create_sync_obj(info)?;
        Ok(MagmaSyncObj { sync_obj })
    }

    pub fn import_sync_obj(&self, info: MagmaImportHandleInfo) -> MagmaResult<MagmaSyncObj> {
        let sync_obj = self.device.clone().import_sync_obj(info)?;
        Ok(MagmaSyncObj { sync_obj })
    }
}

impl MagmaBuffer {
    pub fn map_cpu(&self) -> MagmaResult<MagmaMapping> {
        let mut mapping = self.mapping.lock().unwrap();
        if mapping.is_none() {
            let region = self.buffer.clone().map()?;
            *mapping = Some(region);
        }
        let region = mapping.as_ref().unwrap();
        Ok(MagmaMapping {
            ptr: region.as_ptr() as *mut c_void,
            size: region.size() as u64,
        })
    }

    pub fn unmap_cpu(&self) -> MagmaResult<()> {
        let mut mapping = self.mapping.lock().unwrap();
        *mapping = None;
        Ok(())
    }

    pub fn map(&self) -> MagmaResult<Arc<dyn MappedRegion>> {
        let region = self.buffer.clone().map()?;
        Ok(region)
    }

    pub fn export(&self) -> MagmaResult<MagmaGpuHandle> {
        let handle = self.buffer.export()?;
        Ok(handle)
    }

    pub fn invalidate(
        &self,
        sync_flags: u64,
        ranges: &[MagmaMappedMemoryRange],
    ) -> MagmaResult<()> {
        self.buffer.invalidate(sync_flags, ranges)?;
        Ok(())
    }

    pub fn flush(&self, sync_flags: u64, ranges: &[MagmaMappedMemoryRange]) -> MagmaResult<()> {
        self.buffer.flush(sync_flags, ranges)?;
        Ok(())
    }
}

impl MagmaAddressSpace {
    pub fn map_buffer_gpu(
        &self,
        buffer: &MagmaBuffer,
        buffer_offset: u64,
        gpu_va: u64,
        size: u64,
        flags: MagmaGpuMapFlags,
    ) -> MagmaResult<()> {
        self.address_space
            .map_buffer_gpu(&buffer.buffer, buffer_offset, gpu_va, size, flags)?;
        Ok(())
    }

    pub fn unmap_buffer_gpu(&self, gpu_va: u64, size: u64) -> MagmaResult<()> {
        self.address_space.unmap_buffer_gpu(gpu_va, size)?;
        Ok(())
    }
}

impl MagmaSubmitInfo {
    pub fn new_address_space(address_space: u32, command_va: u64, length: u64, flags: u32) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitInfo as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            flags,
            address_space_info: MagmaSubmitAddressSpaceInfo::new(address_space, command_va, length),
            buffer_info: Default::default(),
            ..Default::default()
        }
    }

    pub fn new_buffer(command_buffer: u32, start_offset: u64, length: u64, flags: u32) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitInfo as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            flags,
            address_space_info: Default::default(),
            buffer_info: MagmaSubmitBufferInfo::new(command_buffer, start_offset, length),
            ..Default::default()
        }
    }

    pub fn address_space_info(&self) -> Option<&MagmaSubmitAddressSpaceInfo> {
        if self.address_space_info.header.stype == MagmaStructureType::SubmitAddressSpaceInfo as u32
        {
            Some(&self.address_space_info)
        } else {
            None
        }
    }

    pub fn buffer_info(&self) -> Option<&MagmaSubmitBufferInfo> {
        if self.buffer_info.header.stype == MagmaStructureType::SubmitBufferInfo as u32 {
            Some(&self.buffer_info)
        } else {
            None
        }
    }

    pub fn sync_info(&self) -> Option<&MagmaSubmitSyncInfo> {
        if self.sync_info.header.stype == MagmaStructureType::SubmitSyncInfo as u32 {
            Some(&self.sync_info)
        } else {
            None
        }
    }

    pub fn set_sync_info(&mut self, sync_info: MagmaSubmitSyncInfo) {
        self.sync_info = sync_info;
    }
}

impl MagmaSubmitSyncInfo {
    pub fn from_sync_objs(
        wait_sync_objs: &[&MagmaSyncObj],
        signal_sync_objs: &[&MagmaSyncObj],
    ) -> Self {
        let mut wait = [0u32; MAGMA_MAX_SYNCOBJS];
        let mut signal = [0u32; MAGMA_MAX_SYNCOBJS];
        let num_wait = wait_sync_objs.len().min(MAGMA_MAX_SYNCOBJS);
        let num_signal = signal_sync_objs.len().min(MAGMA_MAX_SYNCOBJS);
        for (i, obj) in wait_sync_objs.iter().take(num_wait).enumerate() {
            wait[i] = obj.as_raw_handle().unwrap_or(0);
        }
        for (i, obj) in signal_sync_objs.iter().take(num_signal).enumerate() {
            signal[i] = obj.as_raw_handle().unwrap_or(0);
        }
        MagmaSubmitSyncInfo::new(num_wait as u32, num_signal as u32, wait, signal)
    }
}

impl MagmaQueue {
    pub fn submit_command(&self, submit_info: &MagmaSubmitInfo) -> MagmaResult<()> {
        self.queue.submit_command(submit_info)?;
        Ok(())
    }

    pub fn check_status(&self) -> MagmaResult<()> {
        self.queue.check_status()?;
        Ok(())
    }
}
