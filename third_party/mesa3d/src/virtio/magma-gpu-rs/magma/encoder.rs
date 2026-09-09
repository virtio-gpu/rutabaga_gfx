// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.

use crate::protocol::*;
use magma_gpu::util::Writer;

pub struct Encoder<'a> {
    writer: Writer<'a>,
}

impl<'a> Encoder<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {
            writer: Writer::new(buf),
        }
    }

    pub fn from_writer(writer: Writer<'a>) -> Self {
        Self { writer }
    }

    pub fn bytes_written(&self) -> usize {
        self.writer.bytes_written()
    }
    pub fn encode_create_device(&mut self, msg: &CreateDevice) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_get_memory_budget(&mut self, msg: &GetMemoryBudget) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_create_buffer(&mut self, msg: &CreateBuffer) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_create_address_space(&mut self, msg: &CreateAddressSpace) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_create_queue(&mut self, msg: &CreateQueue) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_device_close(&mut self, msg: &DeviceClose) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_physical_device_close(
        &mut self,
        msg: &PhysicalDeviceClose,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_buffer_close(&mut self, msg: &BufferClose) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_queue_close(&mut self, msg: &QueueClose) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_address_space_close(&mut self, msg: &AddressSpaceClose) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_virt_create_render_thread(
        &mut self,
        msg: &VirtCreateRenderThread,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_map_buffer_gpu(&mut self, msg: &MapBufferGpu) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_unmap_buffer_gpu(&mut self, msg: &UnmapBufferGpu) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_submit_command(&mut self, msg: &SubmitCommand) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_create_sync_obj(&mut self, msg: &CreateSyncObj) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_sync_obj_close(&mut self, msg: &SyncObjClose) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_sync_obj_wait(&mut self, msg: &SyncObjWait) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_sync_obj_signal(&mut self, msg: &SyncObjSignal) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_sync_obj_timeline_wait(
        &mut self,
        msg: &SyncObjTimelineWait,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_sync_obj_timeline_signal(
        &mut self,
        msg: &SyncObjTimelineSignal,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_sync_obj_timeline_query(
        &mut self,
        msg: &SyncObjTimelineQuery,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_queue_check_status(&mut self, msg: &QueueCheckStatus) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_virt_ping(&mut self, msg: &VirtPing) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_virt_create_fence(&mut self, msg: &VirtCreateFence) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }

    pub fn encode_resp_create_device(&mut self, msg: &CreateDeviceResp) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_get_memory_budget(
        &mut self,
        msg: &GetMemoryBudgetResp,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_create_buffer(&mut self, msg: &CreateBufferResp) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_create_address_space(
        &mut self,
        msg: &CreateAddressSpaceResp,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_create_queue(&mut self, msg: &CreateQueueResp) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_map_buffer_gpu(&mut self, msg: &MapBufferGpuResp) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_unmap_buffer_gpu(
        &mut self,
        msg: &UnmapBufferGpuResp,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_submit_command(&mut self, msg: &SubmitCommandResp) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_create_sync_obj(&mut self, msg: &CreateSyncObjResp) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_sync_obj_wait(&mut self, msg: &SyncObjWaitResp) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_sync_obj_signal(&mut self, msg: &SyncObjSignalResp) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_sync_obj_timeline_wait(
        &mut self,
        msg: &SyncObjTimelineWaitResp,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_sync_obj_timeline_signal(
        &mut self,
        msg: &SyncObjTimelineSignalResp,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_sync_obj_timeline_query(
        &mut self,
        msg: &SyncObjTimelineQueryResp,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_queue_check_status(
        &mut self,
        msg: &QueueCheckStatusResp,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }

    pub fn encode_create_buffer_info(
        &mut self,
        msg: &MagmaCreateBufferInfo,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_device_create_info(
        &mut self,
        msg: &MagmaDeviceCreateInfo,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_submit_info(&mut self, msg: &MagmaSubmitInfo) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_submit_address_space_info(
        &mut self,
        msg: &MagmaSubmitAddressSpaceInfo,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_submit_buffer_info(
        &mut self,
        msg: &MagmaSubmitBufferInfo,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_submit_sync_info(&mut self, msg: &MagmaSubmitSyncInfo) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_create_sync_obj_info(
        &mut self,
        msg: &MagmaCreateSyncObjInfo,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_sync_properties(&mut self, msg: &MagmaSyncProperties) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_virt_capabilities(&mut self, msg: &MagmaVirtCapabilities) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_physical_device_info(
        &mut self,
        msg: &MagmaPhysicalDeviceInfo,
    ) -> std::io::Result<()> {
        self.writer.write_all(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_msg(&mut self, msg: &MagmaProtocol) -> std::io::Result<()> {
        match msg {
            MagmaProtocol::CreateDevice(msg) => self.encode_create_device(msg),
            MagmaProtocol::GetMemoryBudget(msg) => self.encode_get_memory_budget(msg),
            MagmaProtocol::CreateBuffer(msg) => self.encode_create_buffer(msg),
            MagmaProtocol::CreateAddressSpace(msg) => self.encode_create_address_space(msg),
            MagmaProtocol::CreateQueue(msg) => self.encode_create_queue(msg),
            MagmaProtocol::DeviceClose(msg) => self.encode_device_close(msg),
            MagmaProtocol::PhysicalDeviceClose(msg) => self.encode_physical_device_close(msg),
            MagmaProtocol::BufferClose(msg) => self.encode_buffer_close(msg),
            MagmaProtocol::QueueClose(msg) => self.encode_queue_close(msg),
            MagmaProtocol::AddressSpaceClose(msg) => self.encode_address_space_close(msg),
            MagmaProtocol::VirtCreateRenderThread(msg) => {
                self.encode_virt_create_render_thread(msg)
            }
            MagmaProtocol::MapBufferGpu(msg) => self.encode_map_buffer_gpu(msg),
            MagmaProtocol::UnmapBufferGpu(msg) => self.encode_unmap_buffer_gpu(msg),
            MagmaProtocol::SubmitCommand(msg) => self.encode_submit_command(msg),
            MagmaProtocol::CreateSyncObj(msg) => self.encode_create_sync_obj(msg),
            MagmaProtocol::SyncObjClose(msg) => self.encode_sync_obj_close(msg),
            MagmaProtocol::SyncObjWait(msg) => self.encode_sync_obj_wait(msg),
            MagmaProtocol::SyncObjSignal(msg) => self.encode_sync_obj_signal(msg),
            MagmaProtocol::SyncObjTimelineWait(msg) => self.encode_sync_obj_timeline_wait(msg),
            MagmaProtocol::SyncObjTimelineSignal(msg) => self.encode_sync_obj_timeline_signal(msg),
            MagmaProtocol::SyncObjTimelineQuery(msg) => self.encode_sync_obj_timeline_query(msg),
            MagmaProtocol::QueueCheckStatus(msg) => self.encode_queue_check_status(msg),
            MagmaProtocol::VirtPing(msg) => self.encode_virt_ping(msg),
            MagmaProtocol::VirtCreateFence(msg) => self.encode_virt_create_fence(msg),

            MagmaProtocol::CreateDeviceResp(msg) => self.encode_resp_create_device(msg),
            MagmaProtocol::GetMemoryBudgetResp(msg) => self.encode_resp_get_memory_budget(msg),
            MagmaProtocol::CreateBufferResp(msg) => self.encode_resp_create_buffer(msg),
            MagmaProtocol::CreateAddressSpaceResp(msg) => {
                self.encode_resp_create_address_space(msg)
            }
            MagmaProtocol::CreateQueueResp(msg) => self.encode_resp_create_queue(msg),
            MagmaProtocol::MapBufferGpuResp(msg) => self.encode_resp_map_buffer_gpu(msg),
            MagmaProtocol::UnmapBufferGpuResp(msg) => self.encode_resp_unmap_buffer_gpu(msg),
            MagmaProtocol::SubmitCommandResp(msg) => self.encode_resp_submit_command(msg),
            MagmaProtocol::CreateSyncObjResp(msg) => self.encode_resp_create_sync_obj(msg),
            MagmaProtocol::SyncObjWaitResp(msg) => self.encode_resp_sync_obj_wait(msg),
            MagmaProtocol::SyncObjSignalResp(msg) => self.encode_resp_sync_obj_signal(msg),
            MagmaProtocol::SyncObjTimelineWaitResp(msg) => {
                self.encode_resp_sync_obj_timeline_wait(msg)
            }
            MagmaProtocol::SyncObjTimelineSignalResp(msg) => {
                self.encode_resp_sync_obj_timeline_signal(msg)
            }
            MagmaProtocol::SyncObjTimelineQueryResp(msg) => {
                self.encode_resp_sync_obj_timeline_query(msg)
            }
            MagmaProtocol::QueueCheckStatusResp(msg) => self.encode_resp_queue_check_status(msg),
        }
    }

    #[allow(dead_code)]
    pub fn encode_extensible_struct(&mut self, msg: &MagmaExtensibleStruct) -> std::io::Result<()> {
        match msg {
            MagmaExtensibleStruct::CreateBufferInfo(msg) => self.encode_create_buffer_info(msg),
            MagmaExtensibleStruct::DeviceCreateInfo(msg) => self.encode_device_create_info(msg),
            MagmaExtensibleStruct::SubmitInfo(msg) => self.encode_submit_info(msg),
            MagmaExtensibleStruct::SubmitAddressSpaceInfo(msg) => {
                self.encode_submit_address_space_info(msg)
            }
            MagmaExtensibleStruct::SubmitBufferInfo(msg) => self.encode_submit_buffer_info(msg),
            MagmaExtensibleStruct::SubmitSyncInfo(msg) => self.encode_submit_sync_info(msg),
            MagmaExtensibleStruct::CreateSyncObjInfo(msg) => self.encode_create_sync_obj_info(msg),
            MagmaExtensibleStruct::SyncProperties(msg) => self.encode_sync_properties(msg),
            MagmaExtensibleStruct::VirtCapabilities(msg) => self.encode_virt_capabilities(msg),
            MagmaExtensibleStruct::PhysicalDeviceInfo(msg) => self.encode_physical_device_info(msg),
        }
    }
}
