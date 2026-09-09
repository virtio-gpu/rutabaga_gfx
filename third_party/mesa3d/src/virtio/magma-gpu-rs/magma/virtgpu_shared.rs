// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

#![allow(dead_code)]

use magma_gpu::util::Error as MagmaGpuError;
use magma_gpu::util::Result as MagmaGpuResult;

use crate::encoder::Encoder;
use crate::magma_defines::MagmaCreateBufferInfo;
use crate::magma_defines::MagmaCreateQueueInfo;
use crate::magma_defines::MagmaHeapBudget;
use crate::protocol::*;

pub fn encode_and_submit_create_device<F>(submit: F) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = CreateDevice {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_CREATE_DEVICE,
            size: std::mem::size_of::<CreateDevice>() as u32,
        },
        ..Default::default()
    };
    encoder.encode_create_device(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

pub fn encode_and_submit_get_memory_budget<F>(
    heap_idx: u32,
    submit: F,
) -> MagmaGpuResult<MagmaHeapBudget>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = GetMemoryBudget {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_GET_MEMORY_BUDGET,
            size: std::mem::size_of::<GetMemoryBudget>() as u32,
        },
        device: 0,
        heap_idx,
    };
    encoder.encode_get_memory_budget(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])?;
    Err(MagmaGpuError::Unsupported)
}

pub fn encode_and_submit_create_address_space<F>(submit: F) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = CreateAddressSpace {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_CREATE_ADDRESS_SPACE,
            size: std::mem::size_of::<CreateAddressSpace>() as u32,
        },
        device: 0,
        ..Default::default()
    };
    encoder.encode_create_address_space(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

pub fn encode_and_submit_create_queue<F>(
    address_space: u32,
    info: &MagmaCreateQueueInfo,
    submit: F,
) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = CreateQueue {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_CREATE_QUEUE,
            size: std::mem::size_of::<CreateQueue>() as u32,
        },
        device: 0,
        address_space,
        info: *info,
    };
    encoder.encode_create_queue(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

#[allow(dead_code)]
pub fn encode_and_submit_map_buffer_gpu<F>(
    address_space: u32,
    buffer: u32,
    buffer_offset: u64,
    gpu_va: u64,
    size: u64,
    flags: MagmaGpuMapFlags,
    submit: F,
) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = MapBufferGpu {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_MAP_BUFFER_GPU,
            size: std::mem::size_of::<MapBufferGpu>() as u32,
        },
        address_space,
        buffer,
        buffer_offset,
        gpu_va,
        size,
        flags,
    };
    encoder.encode_map_buffer_gpu(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

#[allow(dead_code)]
pub fn encode_and_submit_unmap_buffer_gpu<F>(
    address_space: u32,
    gpu_va: u64,
    size: u64,
    submit: F,
) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = UnmapBufferGpu {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_UNMAP_BUFFER_GPU,
            size: std::mem::size_of::<UnmapBufferGpu>() as u32,
        },
        address_space,
        _pad0: 0,
        gpu_va,
        size,
    };
    encoder.encode_unmap_buffer_gpu(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

#[allow(dead_code)]
pub fn encode_and_submit_submit_command<F>(
    queue: u32,
    submit_info: &MagmaSubmitInfo,
    submit: F,
) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 512];
    let mut encoder = Encoder::new(&mut buf);
    let req = SubmitCommand {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_SUBMIT_COMMAND,
            size: std::mem::size_of::<SubmitCommand>() as u32,
        },
        queue,
        _pad0: 0,
        submit_info: *submit_info,
    };
    encoder.encode_submit_command(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

pub fn encode_and_submit_create_buffer<F>(
    create_info: &MagmaCreateBufferInfo,
    submit: F,
) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 512];
    let mut encoder = Encoder::new(&mut buf);
    let req = CreateBuffer {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_CREATE_BUFFER,
            size: std::mem::size_of::<CreateBuffer>() as u32,
        },
        device: 0,
        info: *create_info,
        ..Default::default()
    };
    encoder.encode_create_buffer(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

#[allow(dead_code)]
pub fn encode_and_submit_create_sync_obj<F>(
    info: &MagmaCreateSyncObjInfo,
    submit: F,
) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = CreateSyncObj {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_CREATE_SYNC_OBJ,
            size: std::mem::size_of::<CreateSyncObj>() as u32,
        },
        device: 0,
        _pad0: 0,
        info: *info,
    };
    encoder.encode_create_sync_obj(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

pub fn encode_and_submit_sync_obj_wait<F>(
    sync_obj: u32,
    timeout_ns: u64,
    submit: F,
) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = SyncObjWait {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_SYNC_OBJ_WAIT,
            size: std::mem::size_of::<SyncObjWait>() as u32,
        },
        sync_obj,
        _pad0: 0,
        timeout_ns,
    };
    encoder.encode_sync_obj_wait(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

pub fn encode_and_submit_sync_obj_signal<F>(sync_obj: u32, submit: F) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = SyncObjSignal {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_SYNC_OBJ_SIGNAL,
            size: std::mem::size_of::<SyncObjSignal>() as u32,
        },
        sync_obj,
        _padding: 0,
    };
    encoder.encode_sync_obj_signal(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

pub fn encode_and_submit_virt_create_render_thread<F>(
    thread_id: u32,
    ring_blob_id: u32,
    submit: F,
) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = VirtCreateRenderThread {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_VIRT_CREATE_RENDER_THREAD,
            size: std::mem::size_of::<VirtCreateRenderThread>() as u32,
        },
        thread_id,
        ring_blob_id,
    };
    encoder.encode_virt_create_render_thread(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

pub fn encode_and_submit_virt_ping<F>(thread_id: u32, submit: F) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = VirtPing {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_VIRT_PING,
            size: std::mem::size_of::<VirtPing>() as u32,
        },
        thread_id,
        _padding: 0,
    };
    encoder.encode_virt_ping(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}

pub fn encode_and_submit_virt_create_fence<F>(
    ring_idx: u32,
    guest_sync_id: u32,
    submit: F,
) -> MagmaGpuResult<()>
where
    F: FnOnce(&[u8]) -> MagmaGpuResult<()>,
{
    let mut buf = [0u8; 256];
    let mut encoder = Encoder::new(&mut buf);
    let req = VirtCreateFence {
        header: MagmaCommandHeader {
            opcode: MAGMA_OPCODE_VIRT_CREATE_FENCE,
            size: std::mem::size_of::<VirtCreateFence>() as u32,
        },
        ring_idx,
        guest_sync_id,
    };
    encoder.encode_virt_create_fence(&req)?;
    let len = encoder.bytes_written();
    submit(&buf[..len])
}
