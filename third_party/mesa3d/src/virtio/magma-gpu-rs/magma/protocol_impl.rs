// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.

#![allow(clippy::needless_update)]
#![allow(clippy::too_many_arguments)]
use crate::protocol::*;

impl MagmaHeap {
    pub fn new(heap_size: u64, heap_flags: u64) -> Self {
        Self {
            heap_size,
            heap_flags,
            ..Default::default()
        }
    }
}

impl MagmaHeapBudget {
    pub fn new(budget: u64, usage: u64) -> Self {
        Self {
            budget,
            usage,
            ..Default::default()
        }
    }
}

impl MagmaMemoryProperties {
    pub fn new(
        memory_type_count: u32,
        memory_heap_count: u32,
        memory_types: [MagmaMemoryType; MAGMA_MAX_MEMORY_TYPES],
        memory_heaps: [MagmaHeap; MAGMA_MAX_MEMORY_HEAPS],
    ) -> Self {
        Self {
            memory_type_count,
            memory_heap_count,
            memory_types,
            memory_heaps,
            ..Default::default()
        }
    }
}

impl MagmaMemoryType {
    pub fn new(property_flags: u32, heap_idx: u32) -> Self {
        Self {
            property_flags,
            heap_idx,
            ..Default::default()
        }
    }
}

impl MagmaPciBusInfo {
    pub fn new(
        domain: u16,
        subvendor_id: u16,
        subdevice_id: u16,
        revision_id: u8,
        bus: u8,
        device: u8,
        function: u8,
    ) -> Self {
        Self {
            domain,
            subvendor_id,
            subdevice_id,
            revision_id,
            bus,
            device,
            function,
            ..Default::default()
        }
    }
}

impl MagmaCreateQueueInfo {
    pub fn new(queue_family_idx: u32, priority: u32, flags: u32) -> Self {
        Self {
            queue_family_idx,
            priority,
            flags,
            ..Default::default()
        }
    }
}

impl MagmaQueueFamilyProperties {
    pub fn new(queue_flags: MagmaQueueFlags, queue_count: u32) -> Self {
        Self {
            queue_flags,
            queue_count,
            ..Default::default()
        }
    }
}

impl MagmaCreateBufferInfo {
    pub fn new(
        memory_type_idx: u32,
        alignment: u32,
        common_flags: u32,
        vendor_flags: u32,
        size: u64,
    ) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::CreateBufferInfo as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            memory_type_idx,
            alignment,
            common_flags,
            vendor_flags,
            size,
            ..Default::default()
        }
    }
}

impl MagmaDeviceCreateInfo {
    pub fn new(flags: u32) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::DeviceCreateInfo as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            flags,
            ..Default::default()
        }
    }
}

impl MagmaSubmitInfo {
    pub fn new(flags: u32, sync_info: MagmaSubmitSyncInfo) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitInfo as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            flags,
            sync_info,
            ..Default::default()
        }
    }
}

impl MagmaSubmitAddressSpaceInfo {
    pub fn new(address_space: u32, command_va: u64, length: u64) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitAddressSpaceInfo as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            address_space,
            command_va,
            length,
            ..Default::default()
        }
    }
}

impl MagmaSubmitBufferInfo {
    pub fn new(command_buffer: u32, start_offset: u64, length: u64) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitBufferInfo as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            command_buffer,
            start_offset,
            length,
            ..Default::default()
        }
    }
}

impl MagmaSubmitSyncInfo {
    pub fn new(
        num_wait_sync_objs: u32,
        num_signal_sync_objs: u32,
        wait_sync_objs: [u32; MAGMA_MAX_SYNCOBJS],
        signal_sync_objs: [u32; MAGMA_MAX_SYNCOBJS],
    ) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitSyncInfo as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            num_wait_sync_objs,
            num_signal_sync_objs,
            wait_sync_objs,
            signal_sync_objs,
            ..Default::default()
        }
    }
}

impl MagmaCreateSyncObjInfo {
    pub fn new(sync_type: u32, flags: u32, initial_point: u64) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::CreateSyncObjInfo as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            sync_type,
            flags,
            initial_point,
            ..Default::default()
        }
    }
}

impl MagmaSyncProperties {
    pub fn new(capabilities: u32, max_timeline_value_difference: u64) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SyncProperties as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            capabilities,
            max_timeline_value_difference,
            ..Default::default()
        }
    }
}

impl MagmaVirtCapabilities {
    pub fn new(
        capset_version: u32,
        num_physical_devices: u32,
        blob_preference: VirtioGpuBlobPreference,
        devices: [MagmaPhysicalDeviceInfo; MAGMA_MAX_PHYSICAL_DEVICES],
    ) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::VirtCapabilities as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            capset_version,
            num_physical_devices,
            blob_preference,
            devices,
            ..Default::default()
        }
    }
}

impl MagmaPhysicalDeviceInfo {
    pub fn new(
        bus_type: u32,
        vendor_id: MagmaVendorId,
        device_id: u16,
        flags: u32,
        pci_bus_info: MagmaPciBusInfo,
        memory_properties: MagmaMemoryProperties,
        queue_family_count: u32,
        queue_families: [MagmaQueueFamilyProperties; MAGMA_MAX_QUEUES],
    ) -> Self {
        Self {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::PhysicalDeviceInfo as u32,
                size: std::mem::size_of::<Self>() as u32,
                p_next: 0,
            },
            bus_type,
            vendor_id,
            device_id,
            flags,
            pci_bus_info,
            memory_properties,
            queue_family_count,
            queue_families,
            ..Default::default()
        }
    }
}
