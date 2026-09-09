// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use magma_gpu::util::Error as MagmaGpuError;
use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MappedRegion;
use magma_gpu::util::Result as MagmaGpuResult;
use magma_gpu::virtgpu_kumquat::VirtGpuKumquat;

use crate::magma_defines::MagmaCreateBufferInfo;
use crate::magma_defines::MagmaCreateQueueInfo;
use crate::magma_defines::MagmaCreateSyncObjInfo;
use crate::magma_defines::MagmaError;
use crate::magma_defines::MagmaHeapBudget;
use crate::magma_defines::MagmaImportHandleInfo;
use crate::magma_defines::MagmaMappedMemoryRange;
use crate::magma_defines::MagmaResult;
use crate::magma_defines::MagmaSubmitInfo;
use crate::magma_defines::MagmaSyncType;
use crate::protocol::MagmaGpuMapFlags;
use std::sync::Mutex;

use crate::magma_defines::MagmaMemoryProperties;
use crate::magma_defines::MagmaPhysicalDeviceInfo;
use crate::magma_defines::MagmaQueueFamilyProperties;
use crate::sys::platform::PlatformDevice;
use crate::sys::platform::PlatformPhysicalDevice;

pub trait AsVirtGpu {
    #[allow(dead_code)]
    fn as_virtgpu(&self) -> Option<&Mutex<VirtGpuKumquat>> {
        None
    }
}

pub trait GenericPhysicalDevice {
    fn create_device(
        self: Arc<Self>,
        device_info: &MagmaPhysicalDeviceInfo,
    ) -> MagmaGpuResult<Arc<dyn Device>>;

    fn query_memory_properties(&self) -> MagmaGpuResult<MagmaMemoryProperties>;

    fn query_queue_family_properties(&self) -> MagmaGpuResult<Vec<MagmaQueueFamilyProperties>>;
}

pub trait GenericDevice {
    fn get_memory_budget(&self, heap_idx: u32) -> MagmaGpuResult<MagmaHeapBudget>;

    fn create_address_space(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn AddressSpace>>;

    fn create_queue(
        self: Arc<Self>,
        address_space: &Arc<dyn AddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> MagmaGpuResult<Arc<dyn Queue>>;

    fn create_buffer(
        self: Arc<Self>,
        create_info: &MagmaCreateBufferInfo,
    ) -> MagmaGpuResult<Arc<dyn Buffer>>;

    fn import(self: Arc<Self>, _info: MagmaImportHandleInfo) -> MagmaGpuResult<Arc<dyn Buffer>>;

    fn create_sync_obj(
        self: Arc<Self>,
        _info: &MagmaCreateSyncObjInfo,
    ) -> MagmaGpuResult<Arc<dyn SyncObject>> {
        Err(MagmaGpuError::Unsupported)
    }

    fn import_sync_obj(
        self: Arc<Self>,
        _info: MagmaImportHandleInfo,
    ) -> MagmaGpuResult<Arc<dyn SyncObject>> {
        Err(MagmaGpuError::Unsupported)
    }
}

pub trait GenericBuffer {
    fn map(self: Arc<Self>) -> MagmaGpuResult<Arc<dyn MappedRegion>>;

    fn export(&self) -> MagmaGpuResult<MagmaGpuHandle>;

    fn invalidate(&self, sync_flags: u64, ranges: &[MagmaMappedMemoryRange]) -> MagmaGpuResult<()>;

    fn flush(&self, sync_flags: u64, ranges: &[MagmaMappedMemoryRange]) -> MagmaGpuResult<()>;

    fn as_gem_handle(&self) -> Option<u32> {
        None
    }
}

pub trait GenericSyncObject {
    fn get_type(&self) -> MagmaSyncType {
        MagmaSyncType::Binary
    }

    fn is_timeline(&self) -> bool {
        self.get_type() == MagmaSyncType::Timeline
    }

    fn wait(&self, _timeout_ns: u64) -> MagmaResult<()> {
        Err(MagmaError::Unimplemented)
    }

    fn signal(&self) -> MagmaGpuResult<()> {
        Err(MagmaGpuError::Unsupported)
    }

    fn timeline_wait(&self, _point: u64, _timeout_ns: u64, _flags: u32) -> MagmaResult<()> {
        Err(MagmaError::Unimplemented)
    }

    fn timeline_signal(&self, _point: u64) -> MagmaGpuResult<()> {
        Err(MagmaGpuError::Unsupported)
    }

    fn timeline_query(&self) -> MagmaGpuResult<u64> {
        Err(MagmaGpuError::Unsupported)
    }

    fn export_fence(&self) -> MagmaGpuResult<MagmaGpuHandle> {
        Err(MagmaGpuError::Unsupported)
    }

    fn import(&self, _handle: MagmaGpuHandle) -> MagmaGpuResult<()> {
        Err(MagmaGpuError::Unsupported)
    }

    fn as_raw_handle(&self) -> Option<u32> {
        None
    }
}

pub trait GenericAddressSpace {
    fn as_vm_id(&self) -> Option<u32> {
        None
    }

    fn map_buffer_gpu(
        &self,
        _buffer: &Arc<dyn Buffer>,
        _buffer_offset: u64,
        _gpu_va: u64,
        _size: u64,
        _flags: MagmaGpuMapFlags,
    ) -> MagmaGpuResult<()> {
        Err(MagmaGpuError::Unsupported)
    }

    fn unmap_buffer_gpu(&self, _gpu_va: u64, _size: u64) -> MagmaGpuResult<()> {
        Err(MagmaGpuError::Unsupported)
    }
}

pub trait GenericQueue {
    fn submit_command(&self, _submit_info: &MagmaSubmitInfo) -> MagmaGpuResult<()> {
        Err(MagmaGpuError::Unsupported)
    }

    fn check_status(&self) -> MagmaResult<()> {
        Ok(())
    }
}

pub trait PhysicalDevice:
    PlatformPhysicalDevice + AsVirtGpu + GenericPhysicalDevice + Send + Sync
{
}
pub trait Device: GenericDevice + PlatformDevice + Send + Sync {}
pub trait AddressSpace: GenericAddressSpace + Send + Sync {}
pub trait Queue: GenericQueue + Send + Sync {}
pub trait Buffer: GenericBuffer + Send + Sync {}
pub trait SyncObject: GenericSyncObject + Send + Sync {}
