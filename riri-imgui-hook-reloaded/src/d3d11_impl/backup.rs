use windows::Win32::{
    Foundation::RECT,
    Graphics::{
        Dxgi::Common::DXGI_FORMAT,
        Direct3D::{
            D3D_PRIMITIVE_TOPOLOGY,
            D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
        },
        Direct3D11::{
            D3D11_VIEWPORT,
            ID3D11BlendState,
            ID3D11Buffer,
            ID3D11ClassInstance,
            ID3D11DepthStencilState,
            ID3D11DeviceContext,
            ID3D11GeometryShader,
            ID3D11InputLayout,
            ID3D11PixelShader,
            ID3D11RasterizerState,
            ID3D11SamplerState,
            ID3D11ShaderResourceView,
            ID3D11VertexShader
        }
    }
};

// From imgui source code: https://github.com/ocornut/imgui/blob/master/backends/imgui_impl_dx11.cpp#L201
// "Backup DX state that will be modified to restore it afterwards (unfortunately this is very ugly looking and verbose. Close your eyes!)"
// So true bestie
#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct StateBackup {
    context: Option<ID3D11DeviceContext>,
    scissor_rects: RECT,
    viewports: D3D11_VIEWPORT,
    rasterizer_state: Option<ID3D11RasterizerState>,
    blend_state: Option<ID3D11BlendState>,
    blend_factor: f32,
    sample_mask: u32,
    depth_stencil_state: Option<ID3D11DepthStencilState>,
    stencil_ref: u32,
    shader_resource: Vec<Option<ID3D11ShaderResourceView>>,
    sampler: Vec<Option<ID3D11SamplerState>>,
    ps_shader: Option<ID3D11PixelShader>,
    ps_instances: Option<ID3D11ClassInstance>,
    vs_shader: Option<ID3D11VertexShader>,
    vs_instances: Option<ID3D11ClassInstance>,
    constant_buffer: Vec<Option<ID3D11Buffer>>,
    gs_shader: Option<ID3D11GeometryShader>,
    gs_instances: Option<ID3D11ClassInstance>,
    index_buffer: Option<ID3D11Buffer>,
    index_buffer_offset: u32,
    index_buffer_format: DXGI_FORMAT,
    vertex_buffer: Option<ID3D11Buffer>,
    vertex_buffer_offset: u32,
    vertex_buffer_stride: u32,
    topology: D3D_PRIMITIVE_TOPOLOGY,
    input_layout: Option<ID3D11InputLayout>,
}

#[allow(dead_code)]
impl StateBackup {
    pub(crate) unsafe fn backup(context: Option<ID3D11DeviceContext>) -> Self {
        let mut result = Self::default();

        let ctx = context.as_ref().unwrap();
        ctx.RSGetScissorRects(&mut 16, Some(&mut result.scissor_rects));
        ctx.RSGetViewports(&mut 16, Some(&mut result.viewports));
        result.rasterizer_state = match ctx.RSGetState() {
            Ok(v) => Some(v),
            Err(_) => None
        };
        ctx.OMGetBlendState(
            Some(&mut result.blend_state),
            Some(&mut [result.blend_factor; 4]),
            Some(&mut result.sample_mask),
        );
        ctx.OMGetDepthStencilState(
            Some(&mut result.depth_stencil_state), 
            Some(&mut result.stencil_ref)
        );
        ctx.PSGetShaderResources(0, Some(&mut result.shader_resource));
        ctx.PSGetSamplers(0, Some(&mut result.sampler));
        ctx.PSGetShader(&mut result.ps_shader, Some(&mut result.ps_instances), Some(&mut 256));
        ctx.VSGetShader(&mut result.vs_shader, Some(&mut result.vs_instances), Some(&mut 256));
        ctx.VSGetConstantBuffers(0, Some(result.constant_buffer.as_mut_slice()));
        ctx.GSGetShader(&mut result.gs_shader, Some(&mut result.gs_instances), Some(&mut 256));
        result.topology = ctx.IAGetPrimitiveTopology();
        ctx.IAGetIndexBuffer(
            Some(&mut result.index_buffer),
            Some(&mut result.index_buffer_format),
            Some(&mut result.index_buffer_offset),
        );
        ctx.IAGetVertexBuffers(
            0,
            1,
            Some(&mut result.vertex_buffer),
            Some(&mut result.vertex_buffer_stride),
            Some(&mut result.vertex_buffer_offset),
        );
        result.input_layout = match ctx.IAGetInputLayout() {
            Ok(v) => Some(v),
            Err(_) => None
        };
        result.context = context;
        result
    }
    pub(crate) fn restore(mut self) {
        unsafe {
            let ctx = self.context.as_ref().unwrap();
            let inst =
                if self.ps_instances.is_some() { vec![self.ps_instances.take()] } else { vec![] };
            let vinst =
                if self.vs_instances.is_some() { vec![self.vs_instances.take()] } else { vec![] };

            ctx.RSSetScissorRects(Some(&[self.scissor_rects]));
            ctx.RSSetViewports(Some(&[self.viewports]));
            ctx.RSSetState(self.rasterizer_state.as_ref());
            ctx.OMSetBlendState(self.blend_state.as_ref(), Some(&[self.blend_factor; 4]), 0xFFFFFFFF);
            ctx.OMSetDepthStencilState(self.depth_stencil_state.as_ref(), self.stencil_ref);
            ctx.PSSetShaderResources(0, Some(self.shader_resource.as_slice()));
            ctx.PSSetSamplers(0, Some(self.sampler.as_slice()));
            ctx.PSSetShader(self.ps_shader.as_ref(), Some(inst.as_slice()));
            ctx.VSSetShader(self.vs_shader.as_ref(), Some(vinst.as_slice()));
            ctx.VSSetConstantBuffers(0, Some(self.constant_buffer.as_slice()));
            ctx.GSSetShader(self.gs_shader.as_ref(), Some(&[]));
            ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            ctx.IASetIndexBuffer(
                self.index_buffer.as_ref(),
                self.index_buffer_format,
                self.index_buffer_offset,
            );
            ctx.IASetVertexBuffers(
                0,
                1,
                Some(&raw const self.vertex_buffer),
                Some(&raw const self.vertex_buffer_stride),
                Some(&raw const self.vertex_buffer_offset),
            );
            ctx.IASetInputLayout(self.input_layout.as_ref());
        }
    }
}