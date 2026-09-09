// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap as Map;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::RwLock;

use magma_gpu_magma::MagmaAddressSpace;
use magma_gpu_magma::MagmaBuffer;
use magma_gpu_magma::MagmaDevice;
use magma_gpu_magma::MagmaPhysicalDevice;
use magma_gpu_magma::MagmaQueue;
use magma_gpu_magma::MagmaSyncObj;

/// Shared state container for a Magma device context, accessible across all per-queue worker threads.
pub struct MagmaDeviceState {
    pub physical_devices: Vec<MagmaPhysicalDevice>,
    pub device: Mutex<Option<Arc<MagmaDevice>>>,
    pub address_spaces: RwLock<Map<u32, Arc<MagmaAddressSpace>>>,
    pub buffers: RwLock<Map<u32, Arc<MagmaBuffer>>>,
    pub queues: RwLock<Map<u32, Arc<MagmaQueue>>>,
    pub sync_objs: RwLock<Map<u32, Arc<MagmaSyncObj>>>,
    pub object_id: AtomicU32,
    pub cpu_completed_seqno: AtomicU64,
    pub seqno_condvar: Condvar,
    pub seqno_mutex: Mutex<()>,
}

// SAFETY: Underlying kernel driver objects are thread-safe and internal access is
// synchronized via Mutex and RwLock.
unsafe impl Send for MagmaDeviceState {}
unsafe impl Sync for MagmaDeviceState {}

impl MagmaDeviceState {
    pub fn new(physical_devices: Vec<MagmaPhysicalDevice>) -> Self {
        Self {
            physical_devices,
            device: Mutex::new(None),
            address_spaces: RwLock::new(Map::new()),
            buffers: RwLock::new(Map::new()),
            queues: RwLock::new(Map::new()),
            sync_objs: RwLock::new(Map::new()),
            object_id: AtomicU32::new(1),
            cpu_completed_seqno: AtomicU64::new(0),
            seqno_condvar: Condvar::new(),
            seqno_mutex: Mutex::new(()),
        }
    }

    pub fn set_device(&self, device: MagmaDevice) {
        let mut dev_lock = self.device.lock().unwrap();
        *dev_lock = Some(Arc::new(device));
    }

    pub fn get_device(&self) -> Option<Arc<MagmaDevice>> {
        self.device.lock().unwrap().clone()
    }

    pub fn update_cpu_completed_seqno(&self, seqno: u64) {
        let mut prev = self.cpu_completed_seqno.load(Ordering::Relaxed);
        while seqno > prev {
            match self.cpu_completed_seqno.compare_exchange_weak(
                prev,
                seqno,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.seqno_condvar.notify_all();
                    break;
                }
                Err(actual) => prev = actual,
            }
        }
    }

    pub fn wait_for_cpu_seqno(&self, target: u64) {
        if self.cpu_completed_seqno.load(Ordering::Acquire) >= target {
            return;
        }
        let mut guard = self.seqno_mutex.lock().unwrap();
        while self.cpu_completed_seqno.load(Ordering::Acquire) < target {
            guard = self.seqno_condvar.wait(guard).unwrap();
        }
    }
}
