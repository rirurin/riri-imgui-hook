use crate::{
    d3d11_impl::{
        backup::StateBackup,
        buffer::{ IndexBuffer, VertexBuffer },
        devices::DeviceObjects,
        font::{ FONT_TEX_ID, FontObjects },
        shader::{ PixelShader, VertexShader },
    },
    registry::RegistryFlags
};
use glam::{ Mat4, Vec4 };
use imgui::{
    internal::RawWrapper,
    BackendFlags,
    Context as ImContext,
    DrawCmd,
    DrawCmdParams,
    DrawData,
    DrawIdx,
    DrawVert,
    Textures,
    TextureId
};
// use riri_mod_tools_rt::logln;
use std::{
    ffi::c_void,
    mem::MaybeUninit
};
use windows::{
    core::Interface,
    Win32::{
        Foundation::RECT,
        Graphics::{
            Dxgi::IDXGISwapChain,
            Direct3D11::{
                D3D11_MAPPED_SUBRESOURCE,
                D3D11_MAP_WRITE_DISCARD,
                D3D11_VIEWPORT,
                ID3D11Device,
                ID3D11DeviceContext,
                ID3D11RenderTargetView,
                ID3D11ShaderResourceView,
                ID3D11Texture2D,
            }
        }
    }
};

// Adapted from original C# implementation of riri-imgui-hook:
// https://github.com/rirurin/riri.imguihook/blob/master/riri.imguihook/D3D11Hook.cs

pub static DLL_NAMES: [&'static str; 5] = [
    "d3d11.dll\0",
    "d3d11_1.dll\0",
    "d3d11_2.dll\0",
    "d3d11_3.dll\0",
    "d3d11_4.dll\0"
];

#[allow(dead_code)]
#[derive(Debug)]
pub struct D3D11Hook {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    swapchain: IDXGISwapChain,
    render_target_view: Option<ID3D11RenderTargetView>,
    vertex_shader: Option<VertexShader>,
    pixel_shader: Option<PixelShader>,
    device_objects: Option<DeviceObjects>,
    font_data: Option<FontObjects>,
    vertex_buffer: Option<VertexBuffer>,
    index_buffer: Option<IndexBuffer>,
    textures: Textures<ID3D11ShaderResourceView>,
    resized_buffer: bool,
    print_after_resize: bool
}

unsafe impl Send for D3D11Hook {}
unsafe impl Sync for D3D11Hook {}

impl D3D11Hook { 
    pub fn new(ctx: &mut ImContext, swapchain: IDXGISwapChain, flags: RegistryFlags) -> windows::core::Result<Self> {
        let mut new = unsafe { Self::new_blank(ctx, swapchain)}?;
        unsafe { new.create_device_objects(ctx, flags)? }
        Ok(new)
    }
    pub unsafe fn new_blank(ctx: &mut ImContext, swapchain: IDXGISwapChain) -> windows::core::Result<Self> {
        let renderer_name = format!("riri-imgui-hook-d3d11");
        ctx.set_renderer_name(Some(renderer_name));
        let device: ID3D11Device = swapchain.GetDevice()?;
        let context = device.GetImmediateContext()?;
        let io = ctx.io_mut();
        io.backend_flags |= BackendFlags::RENDERER_HAS_VTX_OFFSET;
        Ok(Self {
            device, swapchain, context, 
            render_target_view: None,
            vertex_shader: None, 
            pixel_shader: None, 
            device_objects: None,
            font_data: None, 
            vertex_buffer: None, 
            index_buffer: None,
            textures: Textures::new(),
            resized_buffer: false,
            print_after_resize: false
        })
    }
    // ImGui_ImplDX11_CreateDeviceObjects
    pub unsafe fn create_device_objects(&mut self, ctx: &mut ImContext, flags: RegistryFlags) -> windows::core::Result<()> {
        let back_buffer = self.swapchain.GetBuffer::<ID3D11Texture2D>(0)?;
        self.device.CreateRenderTargetView(&back_buffer, None, Some(&raw mut self.render_target_view))?;
        self.vertex_shader = Some(VertexShader::new(&self.device)?);
        self.pixel_shader = Some(PixelShader::new(&self.device, flags)?);
        self.device_objects = Some(DeviceObjects::new(&self.device)?);
        self.font_data = Some(FontObjects::new(ctx.fonts(), &self.device)?);
        self.vertex_buffer = Some(VertexBuffer::new(&self.device, 0)?);
        self.index_buffer = Some(IndexBuffer::new(&self.device, 0)?);
        Ok(())
    }

    // ImGui_ImplDX11_RenderDrawData
    pub fn render(&mut self, draw_data: &DrawData) -> windows::core::Result<()> {
        unsafe { self.context.OMSetRenderTargets(Some(&[self.render_target_view.clone()]), None) }
        if draw_data.display_size[0] <= 0.0 
        || draw_data.display_size[1] <= 0.0 {
            return Ok(());
        }
        unsafe {
            if self.vertex_buffer.as_ref().unwrap().len() < draw_data.total_vtx_count as usize {
                // logln!(Verbose, "VERTEX BUFFER [ len: {}, cap: {} ]", draw_data.total_vtx_count, self.vertex_buffer.as_ref().unwrap().len());
                self.vertex_buffer = Some(VertexBuffer::new(&self.device, draw_data.total_vtx_count as usize)?);
            }
            if self.index_buffer.as_ref().unwrap().len() < draw_data.total_idx_count as usize {
                // logln!(Verbose, "INDEX BUFFER [ len: {}, cap: {} ]", draw_data.total_idx_count, self.index_buffer.as_ref().unwrap().len());
                self.index_buffer = Some(IndexBuffer::new(&self.device, draw_data.total_idx_count as usize)?);
            }
            let _state_guard = StateBackup::backup(Some(self.context.clone()));
            self.write_buffers(draw_data)?;
            self.setup_render_state(draw_data);
            self.render_impl(draw_data)?;
            _state_guard.restore(); 
        }
        Ok(())
    }

    unsafe fn write_buffers(&mut self, draw_data: &DrawData) -> windows::core::Result<()> {
        let mut vtx_resource: MaybeUninit<D3D11_MAPPED_SUBRESOURCE> = MaybeUninit::uninit();
        let mut idx_resource: MaybeUninit<D3D11_MAPPED_SUBRESOURCE> = MaybeUninit::uninit();
        self.context.Map(self.vertex_buffer.as_ref().unwrap().get_buffer().map(|v| v.into()), 0, D3D11_MAP_WRITE_DISCARD, 0, Some(vtx_resource.as_mut_ptr()))?;
        self.context.Map(self.index_buffer.as_ref().unwrap().get_buffer().map(|v| v.into()), 0, D3D11_MAP_WRITE_DISCARD, 0, Some(idx_resource.as_mut_ptr()))?;
        let mut vtx_dst = std::slice::from_raw_parts_mut(
            vtx_resource.assume_init_ref().pData.cast::<DrawVert>(),
            draw_data.total_vtx_count as usize,
        );
        let mut idx_dst = std::slice::from_raw_parts_mut(
            idx_resource.assume_init_ref().pData.cast::<DrawIdx>(),
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
        self.context.Unmap(self.vertex_buffer.as_ref().unwrap().get_buffer().map(|v| v.into()), 0);
        self.context.Unmap(self.index_buffer.as_ref().unwrap().get_buffer().map(|v| v.into()), 0);
        let mut mapped_resource: MaybeUninit<D3D11_MAPPED_SUBRESOURCE> = MaybeUninit::uninit();
        let vtx_buf = self.vertex_shader.as_mut().unwrap();
        self.context.Map(
            vtx_buf.get_constant_buffer().map(|v| v.into()),
            0, D3D11_MAP_WRITE_DISCARD, 0, 
            Some(mapped_resource.as_mut_ptr())
        )?;
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
        *mapped_resource.assume_init_mut().pData.cast::<Mat4>() = mvp;
        self.context.Unmap(vtx_buf.get_constant_buffer().map(|v| v.into()), 0);
        Ok(())
    }

    unsafe fn setup_render_state(&self, draw_data: &DrawData) {
        let ctx = &self.context;
        let vp = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: draw_data.display_size[0],
            Height: draw_data.display_size[1],
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        let draw_fmt = if size_of::<DrawIdx>() == 2 {
            windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R16_UINT
        } else {
            windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_UINT
        };
        let stride = size_of::<DrawVert>() as u32;
        let blend_factor = 0.0;

        ctx.RSSetViewports(Some(&[vp]));
        ctx.IASetInputLayout(self.vertex_shader.as_ref().unwrap().get_input_layout());
        ctx.IASetVertexBuffers(0, 1, Some(self.vertex_buffer.as_ref().unwrap().get_buffers()), Some(&stride), Some(&0));
        ctx.IASetIndexBuffer(self.index_buffer.as_ref().unwrap().get_buffer(), draw_fmt, 0);
        ctx.IASetPrimitiveTopology(windows::Win32::Graphics::Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        ctx.VSSetShader(self.vertex_shader.as_ref().unwrap().get_shader(), Some(&[]));
        ctx.VSSetConstantBuffers(0, Some(&[self.vertex_shader.as_ref().unwrap().get_constant_buffer_owned()]));
        ctx.PSSetShader(self.pixel_shader.as_ref().unwrap().get_shader(), Some(&[]));
        ctx.PSSetSamplers(0, Some(&[self.font_data.as_ref().unwrap().get_font_sampler_owned()]));
        ctx.GSSetShader(None,Some(&[]));
        ctx.HSSetShader(None,Some(&[]));
        ctx.DSSetShader(None,Some(&[]));
        ctx.CSSetShader(None,Some(&[]));
        ctx.OMSetBlendState(self.device_objects.as_ref().unwrap().get_blend_state(), Some(&[blend_factor; 4]), 0xFFFFFFFF);
        ctx.OMSetDepthStencilState(self.device_objects.as_ref().unwrap().get_depth_stencil_state(), 0);
        ctx.RSSetState(self.device_objects.as_ref().unwrap().get_rasterizer_state());
    }

    unsafe fn render_impl(&self, draw_data: &DrawData) -> windows::core::Result<()> {
        let clip_off = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;
        let mut vertex_offset = 0;
        let mut index_offset = 0;
        let mut last_tex = TextureId::from(FONT_TEX_ID);
        let context = &self.context;
        context.PSSetShaderResources(0, Some(&[self.font_data.as_ref().unwrap().get_font_resource_view()]));
        for draw_list in draw_data.draw_lists() {
            for cmd in draw_list.commands() {
                match cmd {
                    DrawCmd::Elements {
                        count,
                        cmd_params: DrawCmdParams { clip_rect, texture_id, .. },
                    } => {
                        if texture_id != last_tex {
                            /* 
                            let texture = if texture_id.id() == FONT_TEX_ID {
                                self.font_data.as_ref().unwrap().get_font_resource_view()
                            } else {
                                Some(self.textures
                                    .get(texture_id)
                                    .ok_or(DXGI_ERROR_INVALID_CALL)?
                                    .clone())
                            };
                            */
                            let texture = if texture_id.id() == FONT_TEX_ID {
                                self.font_data.as_ref().unwrap().get_font_resource_view()
                            } else {
                                Some(ID3D11ShaderResourceView::from_raw(texture_id.id() as *mut c_void))
                            };
                            context.PSSetShaderResources(0, Some(&[texture]));
                            last_tex = texture_id;
                        }

                        let r = RECT {
                            left: ((clip_rect[0] - clip_off[0]) * clip_scale[0]) as i32,
                            top: ((clip_rect[1] - clip_off[1]) * clip_scale[1]) as i32,
                            right: ((clip_rect[2] - clip_off[0]) * clip_scale[0]) as i32,
                            bottom: ((clip_rect[3] - clip_off[1]) * clip_scale[1]) as i32,
                        };
                        context.RSSetScissorRects(Some(&[r]));
                        context.DrawIndexed(
                            count as u32,
                            index_offset as u32,
                            vertex_offset as i32,
                        );
                        index_offset += count;
                    },
                    DrawCmd::ResetRenderState => self.setup_render_state(draw_data),
                    DrawCmd::RawCallback { callback, raw_cmd } => {
                        callback(draw_list.raw(), raw_cmd)
                    },
                }
            }
            vertex_offset += draw_list.vtx_buffer().len();
        }
        Ok(())
    }

    // ImGui_ImplDX11_InvalidateDeviceObjects
    pub fn invalidate_render_target_view(&mut self, _ctx: &mut ImContext) -> windows::core::Result<()> {
        self.render_target_view = None;
        Ok(())
    }
    pub unsafe fn create_render_target_view(&mut self, _ctx: &mut ImContext) -> windows::core::Result<()> { 
        let back_buffer = self.swapchain.GetBuffer::<ID3D11Texture2D>(0)?;
        self.device.CreateRenderTargetView(&back_buffer, None, Some(&raw mut self.render_target_view))?;
        Ok(())
    }
}