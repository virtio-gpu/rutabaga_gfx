// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use magma_gpu_magma::magma_enumerate_devices;
use magma_gpu_magma::protocol::MagmaVirtCapabilities;
use magma_gpu_magma::MagmaPhysicalDevice;
use zerocopy::IntoBytes;

use crate::magma::context::MagmaVirtioGpuContext;
use crate::magma::sync_thread::MagmaVirtioGpuSyncThread;
use crate::magma::thread::MagmaVirtioGpuThreadPool;
use crate::rutabaga_core::RutabagaComponent;
use crate::rutabaga_core::RutabagaContext;
use crate::rutabaga_utils::RutabagaFenceHandler;
use crate::rutabaga_utils::RutabagaResult;

pub struct MagmaVirtioGpu {
    caps: MagmaVirtCapabilities,
    physical_devices: Vec<MagmaPhysicalDevice>,
    _fence_handler: RutabagaFenceHandler,
    pool: Arc<MagmaVirtioGpuThreadPool>,
    sync_thread: Arc<MagmaVirtioGpuSyncThread>,
}

impl MagmaVirtioGpu {
    /// Initializes the magma component.
    pub fn init(fence_handler: RutabagaFenceHandler) -> RutabagaResult<Box<dyn RutabagaComponent>> {
        let physical_devices = magma_enumerate_devices().unwrap_or_default();
        let mut caps = MagmaVirtCapabilities::default();
        caps.capset_version = 1;
        caps.num_physical_devices = physical_devices.len().min(caps.devices.len()) as u32;

        for (i, pdev) in physical_devices.iter().enumerate() {
            if i >= caps.devices.len() {
                break;
            }
            caps.devices[i] = *pdev.info();
        }

        let pool = MagmaVirtioGpuThreadPool::new();
        let sync_thread = Arc::new(MagmaVirtioGpuSyncThread::new(fence_handler.clone())?);

        Ok(Box::new(MagmaVirtioGpu {
            caps,
            physical_devices,
            _fence_handler: fence_handler,
            pool,
            sync_thread,
        }))
    }
}

impl RutabagaComponent for MagmaVirtioGpu {
    fn get_capset_info(&self, _capset_id: u32) -> (u32, u32) {
        (1u32, std::mem::size_of::<MagmaVirtCapabilities>() as u32)
    }

    fn get_capset(&self, _capset_id: u32, _version: u32) -> Vec<u8> {
        self.caps.as_bytes().to_vec()
    }

    fn create_context(
        &self,
        _ctx_id: u32,
        _context_init: u32,
        context_name: Option<&str>,
        fence_handler: RutabagaFenceHandler,
    ) -> RutabagaResult<Box<dyn RutabagaContext>> {
        Ok(Box::new(MagmaVirtioGpuContext::new(
            context_name,
            fence_handler,
            self.physical_devices.clone(),
            self.pool.clone(),
            self.sync_thread.clone(),
        )))
    }
}
