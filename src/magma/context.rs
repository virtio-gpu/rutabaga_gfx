// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap as Map;
use std::sync::Arc;
use std::sync::Mutex;

use magma_gpu::util::AtomicMemorySentinel;
use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MemoryMapping;
use magma_gpu::util::Reader;
use magma_gpu::util::SharedMemory;
use magma_gpu::util::MAGMA_GPU_HANDLE_TYPE_MEM_SHM;
use magma_gpu_magma::protocol::*;
use magma_gpu_magma::MagmaPhysicalDevice;

use crate::context_common::ContextResource;
use crate::context_common::ContextResources;
use crate::handle::RutabagaHandle;
use crate::magma::decoder::decode;
use crate::magma::device_state::MagmaDeviceState;
use crate::magma::sync_thread::MagmaVirtioGpuSyncThread;
use crate::magma::thread::MagmaRingChannel;
use crate::magma::thread::MagmaVirtioGpuThreadPool;
use crate::resource::RutabagaResource;
use crate::rutabaga_core::RutabagaContext;
use crate::rutabaga_utils::ResourceCreateBlob;
use crate::rutabaga_utils::RutabagaComponentType;
use crate::rutabaga_utils::RutabagaError;
use crate::rutabaga_utils::RutabagaFence;
use crate::rutabaga_utils::RutabagaFenceHandler;
use crate::rutabaga_utils::RutabagaResult;
use crate::rutabaga_utils::RUTABAGA_FLAG_FENCE_HOST_SHAREABLE;
use crate::rutabaga_utils::RUTABAGA_MAP_ACCESS_RW;
use crate::rutabaga_utils::RUTABAGA_MAP_CACHE_CACHED;
use crate::RutabagaIovec;
use magma_gpu_magma::ring::MagmaRingBuffer;

pub struct MagmaVirtioGpuContext {
    _context_name: Option<String>,
    context_resources: ContextResources,
    _fence_handler: RutabagaFenceHandler,
    device_state: Arc<MagmaDeviceState>,
    rings: Mutex<Map<u32, Arc<MagmaRingChannel>>>,
    pool: Arc<MagmaVirtioGpuThreadPool>,
    sync_thread: Arc<MagmaVirtioGpuSyncThread>,
}

impl MagmaVirtioGpuContext {
    pub fn new(
        context_name: Option<&str>,
        fence_handler: RutabagaFenceHandler,
        physical_devices: Vec<MagmaPhysicalDevice>,
        pool: Arc<MagmaVirtioGpuThreadPool>,
        sync_thread: Arc<MagmaVirtioGpuSyncThread>,
    ) -> MagmaVirtioGpuContext {
        MagmaVirtioGpuContext {
            _context_name: context_name.map(String::from),
            context_resources: Arc::new(Mutex::new(Default::default())),
            _fence_handler: fence_handler,
            device_state: Arc::new(MagmaDeviceState::new(physical_devices)),
            rings: Mutex::new(Map::new()),
            pool,
            sync_thread,
        }
    }
}

impl RutabagaContext for MagmaVirtioGpuContext {
    fn context_create_blob(
        &mut self,
        resource_id: u32,
        resource_create_blob: ResourceCreateBlob,
        iovec_opt: Option<Vec<RutabagaIovec>>,
        handle_opt: Option<RutabagaHandle>,
    ) -> RutabagaResult<RutabagaResource> {
        let handle = match handle_opt {
            Some(handle) => Arc::new(handle),
            None => {
                let shm = SharedMemory::new("magma_blob", resource_create_blob.size)?;
                let magma_handle = MagmaGpuHandle {
                    os_handle: shm.into(),
                    handle_type: MAGMA_GPU_HANDLE_TYPE_MEM_SHM,
                };
                Arc::new(RutabagaHandle::from(magma_handle))
            }
        };

        Ok(RutabagaResource {
            resource_id,
            handle: Some(handle),
            blob: true,
            blob_mem: resource_create_blob.blob_mem,
            blob_flags: resource_create_blob.blob_flags,
            map_info: None,
            info_2d: None,
            info_3d: None,
            vulkan_info: None,
            backing_iovecs: iovec_opt,
            component_mask: 1 << (RutabagaComponentType::Magma as u8),
            size: resource_create_blob.size,
            mapping: None,
        })
    }

    fn submit_cmd(
        &mut self,
        commands: &mut [u8],
        _fence_ids: &[u64],
        _shareable_fences: Vec<MagmaGpuHandle>,
    ) -> RutabagaResult<()> {
        let mut reader = Reader::new(commands);
        while let Some(msg) = decode(&mut reader) {
            match msg {
                MagmaProtocol::VirtCreateRenderThread(create_thread) => {
                    let channel_id = create_thread.thread_id;
                    let ring_blob_id = create_thread.ring_blob_id;

                    let resources = self.context_resources.lock().unwrap();
                    let res = resources
                        .get(&ring_blob_id)
                        .ok_or(RutabagaError::InvalidResourceId)?;
                    let handle = res
                        .handle
                        .as_ref()
                        .ok_or(RutabagaError::InvalidResourceId)?;

                    let mesa_handle = handle
                        .as_mesa_handle()
                        .ok_or(RutabagaError::InvalidResourceId)?;

                    let desc1 = mesa_handle
                        .os_handle
                        .try_clone()
                        .map_err(|_| RutabagaError::InvalidResourceId)?;
                    let desc2 = mesa_handle
                        .os_handle
                        .try_clone()
                        .map_err(|_| RutabagaError::InvalidResourceId)?;

                    // Map ring buffer and sentinel word
                    let ring_mapping = MemoryMapping::from_safe_descriptor(
                        desc1,
                        65536,
                        RUTABAGA_MAP_ACCESS_RW | RUTABAGA_MAP_CACHE_CACHED,
                    )?;
                    let sentinel_mapping = MemoryMapping::from_safe_descriptor(
                        desc2,
                        std::mem::size_of::<u32>(),
                        RUTABAGA_MAP_ACCESS_RW | RUTABAGA_MAP_CACHE_CACHED,
                    )?;

                    let ring = MagmaRingBuffer::new(ring_mapping)?;
                    let sentinel = Arc::new(AtomicMemorySentinel::new(sentinel_mapping)?);

                    let channel = Arc::new(MagmaRingChannel::new(
                        channel_id,
                        ring,
                        sentinel,
                        self.device_state.clone(),
                    ));

                    self.rings
                        .lock()
                        .unwrap()
                        .insert(channel_id, channel.clone());
                    self.pool.schedule(channel);
                }
                MagmaProtocol::VirtPing(ping) => {
                    let rings = self.rings.lock().unwrap();
                    if let Some(channel) = rings.get(&ping.thread_id) {
                        channel.ping();
                        self.pool.schedule(channel.clone());
                    } else if ping.thread_id == 0 {
                        for channel in rings.values() {
                            channel.ping();
                            self.pool.schedule(channel.clone());
                        }
                    }
                }
                _ => return Err(RutabagaError::InvalidCommandBuffer),
            }
        }
        Ok(())
    }

    fn attach(&mut self, resource: &mut RutabagaResource) {
        self.context_resources.lock().unwrap().insert(
            resource.resource_id,
            ContextResource {
                handle: resource.handle.clone(),
                backing_iovecs: resource.backing_iovecs.take(),
            },
        );
    }

    fn detach(&mut self, resource: &RutabagaResource) {
        self.context_resources
            .lock()
            .unwrap()
            .remove(&resource.resource_id);
    }

    fn context_create_fence(
        &mut self,
        fence: RutabagaFence,
    ) -> RutabagaResult<Option<MagmaGpuHandle>> {
        if let Some(cpu_ring) = self.rings.lock().unwrap().get(&0).cloned() {
            cpu_ring.wait_until_idle();
        }

        if fence.ring_idx == 0 {
            if fence.flags & RUTABAGA_FLAG_FENCE_HOST_SHAREABLE == 0 {
                self._fence_handler.call(fence);
            }
            return Ok(None);
        }

        let sync_objs = self.device_state.sync_objs.read().unwrap();
        let sync_obj = sync_objs
            .get(&(fence.ring_idx as u32))
            .ok_or(RutabagaError::InvalidResourceId)?;

        let handle = sync_obj
            .export_fence()
            .map_err(|_| RutabagaError::InvalidResourceId)?;

        if fence.flags & RUTABAGA_FLAG_FENCE_HOST_SHAREABLE != 0 {
            let cloned_handle = handle
                .try_clone()
                .map_err(|_| RutabagaError::InvalidResourceId)?;
            self.sync_thread.add_fence(fence, handle)?;
            Ok(Some(cloned_handle))
        } else {
            self.sync_thread.add_fence(fence, handle)?;
            Ok(None)
        }
    }

    fn component_type(&self) -> RutabagaComponentType {
        RutabagaComponentType::Magma
    }
}
