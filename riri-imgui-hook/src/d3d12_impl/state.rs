use crate::d3d12_impl::{
    buffer::{ IndexBuffer, VertexBuffer },
    font::FontObjects,
    pipeline::GraphicsPipeline,
    signature::RootSignature
};
use glam::{ Mat4, Vec4 };
use imgui::{
    BackendFlags,
    Context as ImContext,
    DrawData,
    DrawIdx,
    DrawVert
};
use std::{
    ffi::c_void,
    mem::ManuallyDrop
};
use windows::Win32::Graphics::{
    Direct3D12::{
        D3D12_COMMAND_LIST_TYPE_DIRECT,
        D3D12_CPU_DESCRIPTOR_HANDLE,
        D3D12_DESCRIPTOR_HEAP_DESC,
        D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
        D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
        D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
        D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
        D3D12_RANGE,
        D3D12_RESOURCE_BARRIER,
        D3D12_RESOURCE_BARRIER_0,
        D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
        D3D12_RESOURCE_BARRIER_FLAG_NONE,
        D3D12_RESOURCE_STATE_PRESENT,
        D3D12_RESOURCE_STATE_RENDER_TARGET,
        D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        D3D12_RESOURCE_TRANSITION_BARRIER,
        D3D12_VIEWPORT,
        ID3D12CommandAllocator,
        ID3D12CommandQueue,
        ID3D12DescriptorHeap,
        ID3D12Device,
        ID3D12GraphicsCommandList,
        ID3D12Resource,
    },
    Dxgi::IDXGISwapChain1
};
use riri_mod_tools_rt::logln;

pub static DLL_NAMES: [&'static str; 1] = [ "d3d12.dll\0" ];

#[allow(dead_code)]
#[derive(Debug)]
pub struct FrameContext {
    alloc: Option<ID3D12CommandAllocator>,
    resrc: Option<ID3D12Resource>,
    desc_handle: D3D12_CPU_DESCRIPTOR_HANDLE
}
impl Default for FrameContext {
    fn default() -> Self {
        Self {
            alloc: None,
            resrc: None,
            desc_handle: D3D12_CPU_DESCRIPTOR_HANDLE::default()
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct D3D12Hook {
    device: ID3D12Device,
    frames: Vec<FrameContext>,
    frame_index: usize,
    desc_heap: ID3D12DescriptorHeap,
    cmd_list: ID3D12GraphicsCommandList,
    bb_desc_heap: ID3D12DescriptorHeap,

    root_signature: RootSignature,
    pipeline: GraphicsPipeline,
    font_objects: FontObjects,
    command_queue: ID3D12CommandQueue,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
}

impl D3D12Hook {
    pub unsafe fn new(
        ctx: &mut ImContext, 
        swapchain: IDXGISwapChain1,
        command_queue: ID3D12CommandQueue
    ) -> windows::core::Result<Self> {
        // initialize resources
        let device = swapchain.GetDevice::<ID3D12Device>()?;
        let desc = swapchain.GetDesc1()?;

        // create frames
        let mut frames = Vec::with_capacity(desc.BufferCount as usize);
        for _ in 0..desc.BufferCount {
            frames.push(FrameContext::default())
        }
        // create descriptor heap
        let desc_heap_param = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            NumDescriptors: desc.BufferCount,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
            NodeMask: 0
        };
        let desc_heap = device.CreateDescriptorHeap::<ID3D12DescriptorHeap>(&raw const desc_heap_param)?;
        // make command allocator
        let cmd_alloc = device.CreateCommandAllocator::<ID3D12CommandAllocator>(D3D12_COMMAND_LIST_TYPE_DIRECT)?;
        for frame in &mut frames {
            frame.alloc = Some(cmd_alloc.clone());
        }
        // make command queue

        // make command list
        let cmd_list = device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, Some(&cmd_alloc), None)?;
        // make back buffer description heap
        let desc_heap_param = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            NumDescriptors: desc.BufferCount,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
            NodeMask: 1
        };
        let bb_desc_heap = device.CreateDescriptorHeap::<ID3D12DescriptorHeap>(&raw const desc_heap_param)?;
        // make frame resources
        let rtv_desc_size = device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
        let mut rtv_handle = bb_desc_heap.GetCPUDescriptorHandleForHeapStart();
        for (i, frame) in frames.iter_mut().enumerate() {
            frame.desc_handle = rtv_handle;
            let resrc = swapchain.GetBuffer::<ID3D12Resource>(i as u32)?;
            device.CreateRenderTargetView(Some(&resrc), None, rtv_handle);
            frame.resrc = Some(resrc);
            rtv_handle.ptr += rtv_desc_size as usize;
        }

        // ImGui_ImplDX12_Init
        let renderer_name = format!("riri-imgui-hook-d3d12");
        ctx.set_renderer_name(Some(renderer_name));
        let io = ctx.io_mut();
        io.backend_flags |= BackendFlags::RENDERER_HAS_VTX_OFFSET;
        // ImGui_ImplDX12_CreateDeviceObjects
        let root_signature = RootSignature::new(&device)?;
        let pipeline = GraphicsPipeline::new(&device, root_signature.get())?;
        let font_objects = FontObjects::new(ctx.fonts(), &desc_heap, &device, &command_queue)?;
        let vertex_buffer = VertexBuffer::new(&device, 0)?;
        let index_buffer = IndexBuffer::new(&device, 0)?;
        Ok(Self { 
            device, frames, frame_index: 0, 
            desc_heap, cmd_list, bb_desc_heap,
            root_signature, pipeline, font_objects,
            command_queue, vertex_buffer, index_buffer
        })
    }

    pub fn prepare(&mut self) -> windows::core::Result<()> {
        let frame_len = self.frames.len();
        let curr_frame = &mut self.frames[self.frame_index % frame_len];
        unsafe {
            curr_frame.alloc.as_ref().unwrap().Reset()?;
            let barrier = D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: ManuallyDrop::new(curr_frame.resrc.clone()),
                        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                        StateBefore: D3D12_RESOURCE_STATE_PRESENT,
                        StateAfter: D3D12_RESOURCE_STATE_RENDER_TARGET
                    })
                }
            };
            self.cmd_list.ResourceBarrier(&[barrier]);
            self.cmd_list.OMSetRenderTargets(1, Some(&raw const curr_frame.desc_handle), false.into(), None);
            self.cmd_list.SetDescriptorHeaps(&[Some(self.desc_heap.clone())]);
        }
        Ok(())
    }

    // ImGui_ImplDX12_RenderDrawData
    pub fn render(&mut self, draw_data: &DrawData) -> windows::core::Result<()> {
        logln!(Verbose, "TODO: Render for D3D12!");
        if draw_data.display_size[0] <= 0.0 
        || draw_data.display_size[1] <= 0.0 {
            return Ok(());
        }
        self.frame_index += 1;
        let frame_len = self.frames.len();
        let curr_frame = &mut self.frames[self.frame_index % frame_len];
        unsafe {
            curr_frame.alloc.as_ref().unwrap().Reset()?;
            let barrier = D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: ManuallyDrop::new(curr_frame.resrc.clone()),
                        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                        StateBefore: D3D12_RESOURCE_STATE_PRESENT,
                        StateAfter: D3D12_RESOURCE_STATE_RENDER_TARGET
                    })
                }
            };
            self.cmd_list.ResourceBarrier(&[barrier]);
            self.cmd_list.OMSetRenderTargets(1, Some(&raw const curr_frame.desc_handle), false.into(), None);
            self.cmd_list.SetDescriptorHeaps(&[Some(self.desc_heap.clone())]);

        }
        // Create and grow vertex/index buffers if needed
        unsafe {
            if self.vertex_buffer.len() < draw_data.total_vtx_count as usize {
                self.vertex_buffer = VertexBuffer::new(&self.device, draw_data.total_vtx_count as usize)?;
            }
            if self.index_buffer.len() < draw_data.total_idx_count as usize {
                self.index_buffer = IndexBuffer::new(&self.device, draw_data.total_idx_count as usize)?;
            }
        }
        // Render command lists
        // (Because we merged all buffers into a single one, we maintain our own offset into them)
        Ok(())
    }

    pub unsafe fn upload_buffer_data(&mut self, draw_data: &DrawData) -> windows::core::Result<()> {
        // Upload vertex/index data into a single contiguous GPU buffer
        let mut vtx_resource: *mut c_void = std::ptr::null_mut();
        let mut idx_resource: *mut c_void = std::ptr::null_mut();
        let range = D3D12_RANGE { Begin: 0, End: 0 };
        // During Map() we specify a null read range (as per DX12 API, this is informational and for tooling only)
        self.vertex_buffer.get_resource().Map(0, Some(&raw const range), Some(&raw mut vtx_resource))?;
        self.index_buffer.get_resource().Map(0, Some(&raw const range), Some(&raw mut idx_resource))?;
        let mut vtx_dst = std::slice::from_raw_parts_mut(
            vtx_resource as *mut DrawVert,
            draw_data.total_vtx_count as usize,
        );
        let mut idx_dst = std::slice::from_raw_parts_mut(
            idx_resource as *mut DrawIdx,
            draw_data.total_idx_count as usize,
        );
        for (vbuf, ibuf) in
            draw_data.draw_lists().map(|draw_list| (draw_list.vtx_buffer(), draw_list.idx_buffer()))
        {
            vtx_dst[..vbuf.len()].copy_from_slice(vbuf);
            idx_dst[..ibuf.len()].copy_from_slice(ibuf);
            vtx_dst = &mut vtx_dst[vbuf.len()..];
            idx_dst = &mut idx_dst[ibuf.len()..];
        }
        // During Unmap() we specify the written range (as per DX12 API, this is informational and for tooling only)
        self.vertex_buffer.get_resource().Unmap(0,Some(&raw const range));
        self.index_buffer.get_resource().Unmap(0,Some(&raw const range));
        Ok(())
    }

    pub unsafe fn setup_render_state(&self, draw_data: &DrawData) {
        // Setup render state structure (for callbacks and custom texture bindings)

        // Setup orthographic projection matrix into our constant buffer
        // Our visible imgui space lies from draw_data->DisplayPos (top left) to draw_data->DisplayPos+data_data->DisplaySize (bottom right).
        let l = draw_data.display_pos[0];
        let r = draw_data.display_pos[0] + draw_data.display_size[0];
        let t = draw_data.display_pos[1];
        let b = draw_data.display_pos[1] + draw_data.display_size[1];
        let mvp = Mat4::from_cols(
            Vec4::new(2.0 / (r - l), 0., 0., 0.,),
            Vec4::new(0.0, 2.0 / (t - b), 0.0, 0.0),
            Vec4::new(0.0, 0.0, 0.5, 0.0),
            Vec4::new((r + l) / (l - r), (t + b) / (b - t), 0.5, 1.0),
        );
        // Setup viewport
        let vp = D3D12_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: draw_data.display_size[0],
            Height: draw_data.display_size[1],
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
    }
}