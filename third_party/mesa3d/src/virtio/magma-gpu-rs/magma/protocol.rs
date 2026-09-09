// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.

#![allow(unused_imports)]
#![allow(dead_code)]

use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

pub const MAGMA_OPCODE_CREATE_DEVICE: u32 = 2;
pub const MAGMA_OPCODE_GET_MEMORY_BUDGET: u32 = 3;
pub const MAGMA_OPCODE_CREATE_BUFFER: u32 = 4;
pub const MAGMA_OPCODE_CREATE_ADDRESS_SPACE: u32 = 5;
pub const MAGMA_OPCODE_CREATE_QUEUE: u32 = 6;
pub const MAGMA_OPCODE_DEVICE_CLOSE: u32 = 7;
pub const MAGMA_OPCODE_PHYSICAL_DEVICE_CLOSE: u32 = 8;
pub const MAGMA_OPCODE_BUFFER_CLOSE: u32 = 9;
pub const MAGMA_OPCODE_QUEUE_CLOSE: u32 = 10;
pub const MAGMA_OPCODE_ADDRESS_SPACE_CLOSE: u32 = 11;
pub const MAGMA_OPCODE_VIRT_CREATE_RENDER_THREAD: u32 = 12;
pub const MAGMA_OPCODE_MAP_BUFFER_GPU: u32 = 13;
pub const MAGMA_OPCODE_UNMAP_BUFFER_GPU: u32 = 14;
pub const MAGMA_OPCODE_SUBMIT_COMMAND: u32 = 15;
pub const MAGMA_OPCODE_CREATE_SYNC_OBJ: u32 = 16;
pub const MAGMA_OPCODE_SYNC_OBJ_CLOSE: u32 = 17;
pub const MAGMA_OPCODE_SYNC_OBJ_WAIT: u32 = 18;
pub const MAGMA_OPCODE_SYNC_OBJ_SIGNAL: u32 = 19;
pub const MAGMA_OPCODE_SYNC_OBJ_TIMELINE_WAIT: u32 = 20;
pub const MAGMA_OPCODE_SYNC_OBJ_TIMELINE_SIGNAL: u32 = 21;
pub const MAGMA_OPCODE_SYNC_OBJ_TIMELINE_QUERY: u32 = 22;
pub const MAGMA_OPCODE_QUEUE_CHECK_STATUS: u32 = 26;
pub const MAGMA_OPCODE_VIRT_PING: u32 = 33;
pub const MAGMA_OPCODE_VIRT_CREATE_FENCE: u32 = 34;

pub const MAGMA_OPCODE_RESP_CREATE_DEVICE: u32 = 0x8000_0002;
pub const MAGMA_OPCODE_RESP_GET_MEMORY_BUDGET: u32 = 0x8000_0003;
pub const MAGMA_OPCODE_RESP_CREATE_BUFFER: u32 = 0x8000_0004;
pub const MAGMA_OPCODE_RESP_CREATE_ADDRESS_SPACE: u32 = 0x8000_0005;
pub const MAGMA_OPCODE_RESP_CREATE_QUEUE: u32 = 0x8000_0006;
pub const MAGMA_OPCODE_RESP_MAP_BUFFER_GPU: u32 = 0x8000_000d;
pub const MAGMA_OPCODE_RESP_UNMAP_BUFFER_GPU: u32 = 0x8000_000e;
pub const MAGMA_OPCODE_RESP_SUBMIT_COMMAND: u32 = 0x8000_000f;
pub const MAGMA_OPCODE_RESP_CREATE_SYNC_OBJ: u32 = 0x8000_0010;
pub const MAGMA_OPCODE_RESP_SYNC_OBJ_WAIT: u32 = 0x8000_0012;
pub const MAGMA_OPCODE_RESP_SYNC_OBJ_SIGNAL: u32 = 0x8000_0013;
pub const MAGMA_OPCODE_RESP_SYNC_OBJ_TIMELINE_WAIT: u32 = 0x8000_0014;
pub const MAGMA_OPCODE_RESP_SYNC_OBJ_TIMELINE_SIGNAL: u32 = 0x8000_0015;
pub const MAGMA_OPCODE_RESP_SYNC_OBJ_TIMELINE_QUERY: u32 = 0x8000_0016;
pub const MAGMA_OPCODE_RESP_QUEUE_CHECK_STATUS: u32 = 0x8000_001a;

pub const MAGMA_MAX_SYNCOBJS: usize = 16;
pub const MAGMA_MAX_PHYSICAL_DEVICES: usize = 8;
pub const MAGMA_MAX_MEMORY_HEAPS: usize = 32;
pub const MAGMA_MAX_MEMORY_TYPES: usize = 16;
pub const MAGMA_MAX_QUEUES: usize = 16;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(u16)]
pub enum MagmaVendorId {
    #[default]
    Undefined = 0,
    Amd = 4098,
    Arm = 5045,
    Intel = 32902,
    Qualcomm = 21523,
    VirtGpu = 6900,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(i32)]
pub enum MagmaStatus {
    #[default]
    Success = 0,
    InternalError = -1,
    InvalidArgs = -2,
    AccessDenied = -3,
    MemoryError = -4,
    ContextKilled = -5,
    TimedOut = -6,
    Unimplemented = -7,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(i32)]
pub enum VirtioGpuBlobPreference {
    #[default]
    Undefined = 0,
    Guest = 1,
    Host = 2,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(u32)]
pub enum MagmaSyncObjType {
    #[default]
    Binary = 0,
    Timeline = 1,
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaMemoryProperty(u32);
bitflags! {
    impl MagmaMemoryProperty: u32 {
        const DeviceLocalBit = 1;
        const HostVisibleBit = 2;
        const HostCoherentBit = 4;
        const HostCachedBit = 8;
        const LazilyAllocatedBit = 16;
        const ProtectedBit = 32;
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaQueueFlags(u32);
bitflags! {
    impl MagmaQueueFlags: u32 {
        const Graphics = 1;
        const Compute = 2;
        const Transfer = 4;
        const SparseBinding = 8;
        const Protected = 16;
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaGpuMapFlags(u64);
bitflags! {
    impl MagmaGpuMapFlags: u64 {
        const Read = 1;
        const Write = 2;
        const Execute = 4;
        const GrowUp = 8;
        const GrowDown = 16;
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaBufferFlag(u32);
bitflags! {
    impl MagmaBufferFlag: u32 {
        const External = 1;
        const Scanout = 2;
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaSyncCapability(u32);
bitflags! {
    impl MagmaSyncCapability: u32 {
        const Binary = 1;
        const Timeline = 2;
        const CpuWait = 4;
        const CpuSignal = 8;
        const ExportSyncFile = 16;
        const ImportSyncFile = 32;
    }
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaHeap {
    pub heap_size: u64,
    pub heap_flags: u64,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaHeapBudget {
    pub budget: u64,
    pub usage: u64,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaMemoryProperties {
    pub memory_type_count: u32,
    pub memory_heap_count: u32,
    pub memory_types: [MagmaMemoryType; MAGMA_MAX_MEMORY_TYPES],
    pub memory_heaps: [MagmaHeap; MAGMA_MAX_MEMORY_HEAPS],
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaMemoryType {
    pub property_flags: u32,
    pub heap_idx: u32,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaPciBusInfo {
    pub domain: u16,
    pub subvendor_id: u16,
    pub subdevice_id: u16,
    pub revision_id: u8,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub _padding: [u8; 6],
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaCreateQueueInfo {
    pub queue_family_idx: u32,
    pub priority: u32,
    pub flags: u32,
    pub _padding: u32,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaQueueFamilyProperties {
    pub queue_flags: MagmaQueueFlags,
    pub queue_count: u32,
}

pub const MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO: u32 = 1;
pub const MAGMA_STRUCTURE_TYPE_DEVICE_CREATE_INFO: u32 = 2;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_INFO: u32 = 3;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO: u32 = 4;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_BUFFER_INFO: u32 = 5;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO: u32 = 6;
pub const MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO: u32 = 7;
pub const MAGMA_STRUCTURE_TYPE_SYNC_PROPERTIES: u32 = 8;
pub const MAGMA_STRUCTURE_TYPE_VIRT_CAPABILITIES: u32 = 65537;
pub const MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO: u32 = 65538;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(u32)]
pub enum MagmaStructureType {
    CreateBufferInfo = 1,
    DeviceCreateInfo = 2,
    SubmitInfo = 3,
    SubmitAddressSpaceInfo = 4,
    SubmitBufferInfo = 5,
    SubmitSyncInfo = 6,
    CreateSyncObjInfo = 7,
    SyncProperties = 8,
    VirtCapabilities = 65537,
    PhysicalDeviceInfo = 65538,
}

#[derive(Debug, Default, Clone, Copy, FromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaStructureTypeHeader {
    pub stype: u32,
    pub size: u32,
    pub p_next: u64,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaCreateBufferInfo {
    pub header: MagmaStructureTypeHeader,
    pub memory_type_idx: u32,
    pub alignment: u32,
    pub common_flags: u32,
    pub vendor_flags: u32,
    pub size: u64,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaDeviceCreateInfo {
    pub header: MagmaStructureTypeHeader,
    pub flags: u32,
    pub _padding: u32,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaSubmitInfo {
    pub header: MagmaStructureTypeHeader,
    pub flags: u32,
    pub _pad0: u32,
    pub sync_info: MagmaSubmitSyncInfo,
    pub address_space_info: MagmaSubmitAddressSpaceInfo,
    pub buffer_info: MagmaSubmitBufferInfo,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaSubmitAddressSpaceInfo {
    pub header: MagmaStructureTypeHeader,
    pub address_space: u32,
    pub _pad0: u32,
    pub command_va: u64,
    pub length: u64,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaSubmitBufferInfo {
    pub header: MagmaStructureTypeHeader,
    pub command_buffer: u32,
    pub _pad0: u32,
    pub start_offset: u64,
    pub length: u64,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaSubmitSyncInfo {
    pub header: MagmaStructureTypeHeader,
    pub num_wait_sync_objs: u32,
    pub num_signal_sync_objs: u32,
    pub wait_sync_objs: [u32; MAGMA_MAX_SYNCOBJS],
    pub signal_sync_objs: [u32; MAGMA_MAX_SYNCOBJS],
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaCreateSyncObjInfo {
    pub header: MagmaStructureTypeHeader,
    pub sync_type: u32,
    pub flags: u32,
    pub initial_point: u64,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaSyncProperties {
    pub header: MagmaStructureTypeHeader,
    pub capabilities: u32,
    pub _pad0: u32,
    pub max_timeline_value_difference: u64,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaVirtCapabilities {
    pub header: MagmaStructureTypeHeader,
    pub capset_version: u32,
    pub num_physical_devices: u32,
    pub blob_preference: VirtioGpuBlobPreference,
    pub _pad0: u32,
    pub devices: [MagmaPhysicalDeviceInfo; MAGMA_MAX_PHYSICAL_DEVICES],
    pub physical_device_info: MagmaPhysicalDeviceInfo,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaPhysicalDeviceInfo {
    pub header: MagmaStructureTypeHeader,
    pub bus_type: u32,
    pub vendor_id: MagmaVendorId,
    pub device_id: u16,
    pub flags: u32,
    pub pci_bus_info: MagmaPciBusInfo,
    pub _pad0: u32,
    pub memory_properties: MagmaMemoryProperties,
    pub queue_family_count: u32,
    pub queue_families: [MagmaQueueFamilyProperties; MAGMA_MAX_QUEUES],
    pub _padding: u32,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum MagmaExtensibleStruct {
    CreateBufferInfo(MagmaCreateBufferInfo),
    DeviceCreateInfo(MagmaDeviceCreateInfo),
    SubmitInfo(MagmaSubmitInfo),
    SubmitAddressSpaceInfo(MagmaSubmitAddressSpaceInfo),
    SubmitBufferInfo(MagmaSubmitBufferInfo),
    SubmitSyncInfo(MagmaSubmitSyncInfo),
    CreateSyncObjInfo(MagmaCreateSyncObjInfo),
    SyncProperties(MagmaSyncProperties),
    VirtCapabilities(MagmaVirtCapabilities),
    PhysicalDeviceInfo(MagmaPhysicalDeviceInfo),
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, FromBytes, IntoBytes, Immutable)]
pub struct MagmaCommandHeader {
    pub opcode: u32,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct CreateDevice {
    pub header: MagmaCommandHeader,
    pub physical_device: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct CreateDeviceResp {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct GetMemoryBudget {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub heap_idx: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct GetMemoryBudgetResp {
    pub header: MagmaCommandHeader,
    pub budget: MagmaHeapBudget,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct CreateBuffer {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub _pad0: u32,
    pub info: MagmaCreateBufferInfo,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct CreateBufferResp {
    pub header: MagmaCommandHeader,
    pub buffer_out: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct CreateAddressSpace {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct CreateAddressSpaceResp {
    pub header: MagmaCommandHeader,
    pub address_space: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct CreateQueue {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub address_space: u32,
    pub info: MagmaCreateQueueInfo,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct CreateQueueResp {
    pub header: MagmaCommandHeader,
    pub queue: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct DeviceClose {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct PhysicalDeviceClose {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct BufferClose {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct QueueClose {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct AddressSpaceClose {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct VirtCreateRenderThread {
    pub header: MagmaCommandHeader,
    pub thread_id: u32,
    pub ring_blob_id: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct MapBufferGpu {
    pub header: MagmaCommandHeader,
    pub address_space: u32,
    pub buffer: u32,
    pub buffer_offset: u64,
    pub gpu_va: u64,
    pub size: u64,
    pub flags: MagmaGpuMapFlags,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct MapBufferGpuResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct UnmapBufferGpu {
    pub header: MagmaCommandHeader,
    pub address_space: u32,
    pub _pad0: u32,
    pub gpu_va: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct UnmapBufferGpuResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SubmitCommand {
    pub header: MagmaCommandHeader,
    pub queue: u32,
    pub _pad0: u32,
    pub submit_info: MagmaSubmitInfo,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SubmitCommandResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct CreateSyncObj {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub _pad0: u32,
    pub info: MagmaCreateSyncObjInfo,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct CreateSyncObjResp {
    pub header: MagmaCommandHeader,
    pub sync_obj: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjClose {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjWait {
    pub header: MagmaCommandHeader,
    pub sync_obj: u32,
    pub _pad0: u32,
    pub timeout_ns: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjWaitResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjSignal {
    pub header: MagmaCommandHeader,
    pub sync_obj: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjSignalResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjTimelineWait {
    pub header: MagmaCommandHeader,
    pub sync_obj: u32,
    pub _pad0: u32,
    pub point: u64,
    pub timeout_ns: u64,
    pub flags: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjTimelineWaitResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjTimelineSignal {
    pub header: MagmaCommandHeader,
    pub sync_obj: u32,
    pub _pad0: u32,
    pub point: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjTimelineSignalResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjTimelineQuery {
    pub header: MagmaCommandHeader,
    pub sync_obj: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SyncObjTimelineQueryResp {
    pub header: MagmaCommandHeader,
    pub point_out: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct QueueCheckStatus {
    pub header: MagmaCommandHeader,
    pub queue: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct QueueCheckStatusResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct VirtPing {
    pub header: MagmaCommandHeader,
    pub thread_id: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct VirtCreateFence {
    pub header: MagmaCommandHeader,
    pub ring_idx: u32,
    pub guest_sync_id: u32,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum MagmaProtocol {
    CreateDevice(CreateDevice),
    GetMemoryBudget(GetMemoryBudget),
    CreateBuffer(CreateBuffer),
    CreateAddressSpace(CreateAddressSpace),
    CreateQueue(CreateQueue),
    DeviceClose(DeviceClose),
    PhysicalDeviceClose(PhysicalDeviceClose),
    BufferClose(BufferClose),
    QueueClose(QueueClose),
    AddressSpaceClose(AddressSpaceClose),
    VirtCreateRenderThread(VirtCreateRenderThread),
    MapBufferGpu(MapBufferGpu),
    UnmapBufferGpu(UnmapBufferGpu),
    SubmitCommand(SubmitCommand),
    CreateSyncObj(CreateSyncObj),
    SyncObjClose(SyncObjClose),
    SyncObjWait(SyncObjWait),
    SyncObjSignal(SyncObjSignal),
    SyncObjTimelineWait(SyncObjTimelineWait),
    SyncObjTimelineSignal(SyncObjTimelineSignal),
    SyncObjTimelineQuery(SyncObjTimelineQuery),
    QueueCheckStatus(QueueCheckStatus),
    VirtPing(VirtPing),
    VirtCreateFence(VirtCreateFence),

    CreateDeviceResp(CreateDeviceResp),
    GetMemoryBudgetResp(GetMemoryBudgetResp),
    CreateBufferResp(CreateBufferResp),
    CreateAddressSpaceResp(CreateAddressSpaceResp),
    CreateQueueResp(CreateQueueResp),
    MapBufferGpuResp(MapBufferGpuResp),
    UnmapBufferGpuResp(UnmapBufferGpuResp),
    SubmitCommandResp(SubmitCommandResp),
    CreateSyncObjResp(CreateSyncObjResp),
    SyncObjWaitResp(SyncObjWaitResp),
    SyncObjSignalResp(SyncObjSignalResp),
    SyncObjTimelineWaitResp(SyncObjTimelineWaitResp),
    SyncObjTimelineSignalResp(SyncObjTimelineSignalResp),
    SyncObjTimelineQueryResp(SyncObjTimelineQueryResp),
    QueueCheckStatusResp(QueueCheckStatusResp),
}
