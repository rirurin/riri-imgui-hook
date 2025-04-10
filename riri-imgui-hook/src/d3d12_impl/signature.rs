use windows::Win32::Graphics::{
    Direct3D::ID3DBlob,
    Direct3D12::{
        D3D12_COMPARISON_FUNC_ALWAYS,
        D3D12_DESCRIPTOR_RANGE,
        D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
        D3D12_FILTER_MIN_MAG_MIP_LINEAR,
        D3D12_ROOT_PARAMETER,
        D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS,
        D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
        D3D12_ROOT_SIGNATURE_DESC,
        D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
        D3D12_ROOT_SIGNATURE_FLAG_DENY_HULL_SHADER_ROOT_ACCESS,
        D3D12_ROOT_SIGNATURE_FLAG_DENY_DOMAIN_SHADER_ROOT_ACCESS,
        D3D12_ROOT_SIGNATURE_FLAG_DENY_GEOMETRY_SHADER_ROOT_ACCESS,
        D3D_ROOT_SIGNATURE_VERSION_1,
        D3D12_SHADER_VISIBILITY_PIXEL,
        D3D12_SHADER_VISIBILITY_VERTEX,
        D3D12_STATIC_BORDER_COLOR_TRANSPARENT_BLACK,
        D3D12_STATIC_SAMPLER_DESC,
        D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
        D3D12SerializeRootSignature,
        ID3D12Device,
        ID3D12RootSignature
    }
};

#[derive(Debug)]
pub struct RootSignature(ID3D12RootSignature);
impl RootSignature {
    pub fn new(device: &ID3D12Device) -> windows::core::Result<Self> {
        let desc_range = D3D12_DESCRIPTOR_RANGE {
            RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
            NumDescriptors: 1,
            BaseShaderRegister: 0,
            RegisterSpace: 0,
            OffsetInDescriptorsFromTableStart: 0
        };
        let mut params: [D3D12_ROOT_PARAMETER; 2] = [D3D12_ROOT_PARAMETER::default(); 2];
        params[0].ParameterType = D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS;
        params[0].Anonymous.Constants.ShaderRegister = 0;
        params[0].Anonymous.Constants.RegisterSpace = 0;
        params[0].Anonymous.Constants.Num32BitValues = 16;
        params[0].ShaderVisibility = D3D12_SHADER_VISIBILITY_VERTEX;

        params[1].ParameterType = D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE;
        params[1].Anonymous.DescriptorTable.NumDescriptorRanges = 1;
        params[1].Anonymous.DescriptorTable.pDescriptorRanges = &raw const desc_range;
        params[1].ShaderVisibility = D3D12_SHADER_VISIBILITY_PIXEL;

        // Bilinear sampling is required by default. Set 'io.Fonts->Flags |= ImFontAtlasFlags_NoBakedLines' or 
        // 'style.AntiAliasedLinesUseTex = false' to allow point/nearest sampling.
        let mut static_sampler = D3D12_STATIC_SAMPLER_DESC::default();
        static_sampler.Filter = D3D12_FILTER_MIN_MAG_MIP_LINEAR;
        static_sampler.AddressU = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
        static_sampler.AddressV = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
        static_sampler.AddressW = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
        static_sampler.MipLODBias = 0.;
        static_sampler.MaxAnisotropy = 0;
        static_sampler.ComparisonFunc = D3D12_COMPARISON_FUNC_ALWAYS;
        static_sampler.BorderColor = D3D12_STATIC_BORDER_COLOR_TRANSPARENT_BLACK;
        static_sampler.MinLOD = 0.;
        static_sampler.MaxLOD = 0.;
        static_sampler.ShaderRegister = 0;
        static_sampler.RegisterSpace = 0;
        static_sampler.ShaderVisibility = D3D12_SHADER_VISIBILITY_PIXEL;

        let mut root_signature = D3D12_ROOT_SIGNATURE_DESC::default();
        root_signature.NumParameters = params.len() as u32;
        root_signature.pParameters = params.as_ptr();
        root_signature.NumStaticSamplers = 1;
        root_signature.pStaticSamplers = &raw const static_sampler;
        root_signature.Flags = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT
        | D3D12_ROOT_SIGNATURE_FLAG_DENY_DOMAIN_SHADER_ROOT_ACCESS
        | D3D12_ROOT_SIGNATURE_FLAG_DENY_GEOMETRY_SHADER_ROOT_ACCESS
        | D3D12_ROOT_SIGNATURE_FLAG_DENY_HULL_SHADER_ROOT_ACCESS;
        let mut root_blob: Option<ID3DBlob> = None;
        unsafe {
            D3D12SerializeRootSignature(
            &raw const root_signature, 
            D3D_ROOT_SIGNATURE_VERSION_1, 
            &raw mut root_blob, 
            None)?;
            let blob_slice = std::slice::from_raw_parts(
                root_blob.as_ref().unwrap().GetBufferPointer() as *const u8, 
                root_blob.as_ref().unwrap().GetBufferSize());
            Ok(Self(device.CreateRootSignature::<ID3D12RootSignature>(0, blob_slice)?))
        }
    }

    pub fn get(&self) -> &ID3D12RootSignature { &self.0 }
    pub fn get_mut(&mut self) -> &mut ID3D12RootSignature { &mut self.0 }
}