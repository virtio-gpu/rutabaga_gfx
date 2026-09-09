// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

mod magma;
mod magma_defines;
mod magma_kumquat;
mod sys;
mod traits;
mod virtgpu_shared;

pub mod encoder;
pub mod protocol;
pub mod protocol_impl;
pub mod ring;

pub use magma_defines::*;

pub use magma::magma_enumerate_devices;
pub use magma::MagmaAddressSpace;
pub use magma::MagmaBuffer;
pub use magma::MagmaDevice;
pub use magma::MagmaHandle;
pub use magma::MagmaMapping;
pub use magma::MagmaPhysicalDevice;
pub use magma::MagmaQueue;
pub use magma::MagmaSyncObj;
pub use magma_gpu::util::Handle;
