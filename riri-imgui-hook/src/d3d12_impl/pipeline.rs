use std::{
    ffi::c_void,
    mem::ManuallyDrop
};
use windows::{
    core::PCSTR,
    Win32::Graphics::{
        Dxgi::Common::{
            DXGI_FORMAT_R32G32_FLOAT,
            DXGI_FORMAT_R8G8B8A8_UNORM
        },
        Direct3D12::{
            D3D12_BLEND_INV_SRC_ALPHA,
            D3D12_BLEND_SRC_ALPHA,
            D3D12_BLEND_ONE,
            D3D12_BLEND_OP_ADD,
            D3D12_COMPARISON_FUNC_ALWAYS,
            D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
            D3D12_COLOR_WRITE_ENABLE_ALL,
            D3D12_CULL_MODE_NONE,
            D3D12_DEFAULT_DEPTH_BIAS,
            D3D12_DEFAULT_DEPTH_BIAS_CLAMP,
            D3D12_DEFAULT_SLOPE_SCALED_DEPTH_BIAS,
            D3D12_DEPTH_WRITE_MASK_ALL,
            D3D12_FILL_MODE_SOLID,
            D3D12_GRAPHICS_PIPELINE_STATE_DESC,
            D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
            D3D12_INPUT_ELEMENT_DESC,
            D3D12_INPUT_LAYOUT_DESC,
            D3D12_PIPELINE_STATE_FLAG_NONE,
            D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            D3D12_SHADER_BYTECODE,
            D3D12_STENCIL_OP_KEEP,
            ID3D12Device,
            ID3D12PipelineState,
            ID3D12RootSignature
        }
    }
};

static VERTEX_SHADER: &'static [u8] = include_bytes!("vs.dxbc");
static PIXEL_SHADER: &'static [u8] = include_bytes!("ps.dxbc");

#[derive(Debug)]
pub struct GraphicsPipeline {
    _impl: ID3D12PipelineState
}
impl GraphicsPipeline {
    pub unsafe fn new(
        device: &ID3D12Device, 
        root: &ID3D12RootSignature
    ) -> windows::core::Result<Self> {
        let mut pipeline = D3D12_GRAPHICS_PIPELINE_STATE_DESC::default();
        pipeline.NodeMask = 1;
        pipeline.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
        pipeline.pRootSignature = ManuallyDrop::new(Some(root.clone()));
        pipeline.SampleMask = u32::MAX;
        pipeline.NumRenderTargets = 1;
        pipeline.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
        pipeline.SampleDesc.Count = 1;
        pipeline.Flags = D3D12_PIPELINE_STATE_FLAG_NONE;
        // Create vertex + pixel shader 
        let vertex_shader = D3D12_SHADER_BYTECODE {
            pShaderBytecode: VERTEX_SHADER.as_ptr() as *const c_void,
            BytecodeLength: VERTEX_SHADER.len()
        };
        let pixel_shader = D3D12_SHADER_BYTECODE {
            pShaderBytecode: PIXEL_SHADER.as_ptr() as *const c_void, 
            BytecodeLength: PIXEL_SHADER.len()
        };

        let local_layout = [
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(b"POSITION\0".as_ptr()),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 0,
                InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(b"TEXCOORD\0".as_ptr()),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 8,
                InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR("COLOR\0".as_ptr()),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                InputSlot: 0,
                AlignedByteOffset: 16,
                InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];
        pipeline.InputLayout = D3D12_INPUT_LAYOUT_DESC {
            pInputElementDescs: local_layout.as_ptr(),
            NumElements: local_layout.len() as u32
        };
        pipeline.VS = vertex_shader;
        pipeline.PS = pixel_shader;
        // Create the blending setup
        let blend = &mut pipeline.BlendState;
        blend.AlphaToCoverageEnable = false.into();
        blend.RenderTarget[0].BlendEnable = true.into();
        blend.RenderTarget[0].SrcBlend = D3D12_BLEND_SRC_ALPHA;
        blend.RenderTarget[0].DestBlend = D3D12_BLEND_INV_SRC_ALPHA;
        blend.RenderTarget[0].BlendOp = D3D12_BLEND_OP_ADD;
        blend.RenderTarget[0].SrcBlendAlpha = D3D12_BLEND_ONE;
        blend.RenderTarget[0].DestBlendAlpha = D3D12_BLEND_INV_SRC_ALPHA;
        blend.RenderTarget[0].BlendOpAlpha = D3D12_BLEND_OP_ADD;
        blend.RenderTarget[0].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8;
        // Create the rasterizer state
        let raster = &mut pipeline.RasterizerState;
        raster.FillMode = D3D12_FILL_MODE_SOLID;
        raster.CullMode = D3D12_CULL_MODE_NONE;
        raster.FrontCounterClockwise = false.into();
        raster.DepthBias = D3D12_DEFAULT_DEPTH_BIAS;
        raster.DepthBiasClamp = D3D12_DEFAULT_DEPTH_BIAS_CLAMP;
        raster.SlopeScaledDepthBias = D3D12_DEFAULT_SLOPE_SCALED_DEPTH_BIAS;
        raster.DepthClipEnable = true.into();
        raster.MultisampleEnable = false.into();
        raster.AntialiasedLineEnable = false.into();
        raster.ForcedSampleCount = 0;
        raster.ConservativeRaster = D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF;
        // Create depth-stencil State
        let ds = &mut pipeline.DepthStencilState;
        ds.DepthEnable = false.into();
        ds.DepthWriteMask = D3D12_DEPTH_WRITE_MASK_ALL;
        ds.DepthFunc = D3D12_COMPARISON_FUNC_ALWAYS;
        ds.StencilEnable = false.into();
        ds.FrontFace.StencilFailOp = D3D12_STENCIL_OP_KEEP;
        ds.FrontFace.StencilDepthFailOp = D3D12_STENCIL_OP_KEEP;
        ds.FrontFace.StencilPassOp = D3D12_STENCIL_OP_KEEP;
        ds.FrontFace.StencilFunc = D3D12_COMPARISON_FUNC_ALWAYS;
        ds.BackFace = ds.FrontFace;

        let _impl = device.CreateGraphicsPipelineState::<ID3D12PipelineState>(&raw const pipeline)?;
        Ok(Self { _impl })
    }
}