use imgui::{ FontAtlas, TextureId };
use std::{
    ffi::c_void,
    mem::ManuallyDrop
};
use windows::Win32::{
    Foundation::CloseHandle,
    Graphics::{
        Dxgi::Common::{
            DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_UNKNOWN,
        },
        Direct3D12::{
            D3D12_COMMAND_LIST_TYPE_DIRECT,
            D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
            D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
            D3D12_FENCE_FLAG_NONE,
            D3D12_HEAP_FLAG_NONE,
            D3D12_HEAP_PROPERTIES,
            D3D12_HEAP_TYPE_DEFAULT,
            D3D12_HEAP_TYPE_UPLOAD,
            D3D12_MEMORY_POOL_UNKNOWN,
            D3D12_RANGE,
            D3D12_RESOURCE_BARRIER,
            D3D12_RESOURCE_BARRIER_0,
            D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
            D3D12_RESOURCE_BARRIER_FLAG_NONE,
            D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            D3D12_RESOURCE_DESC,
            D3D12_RESOURCE_DIMENSION_BUFFER,
            D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            D3D12_RESOURCE_FLAG_NONE,
            D3D12_RESOURCE_STATE_COPY_DEST,
            D3D12_RESOURCE_STATE_GENERIC_READ,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            D3D12_RESOURCE_TRANSITION_BARRIER,
            D3D12_SHADER_RESOURCE_VIEW_DESC,
            D3D12_SRV_DIMENSION_TEXTURE2D,
            D3D12_TEXTURE_COPY_LOCATION,
            D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            D3D12_TEXTURE_LAYOUT_UNKNOWN,
            D3D12_TEXTURE_DATA_PITCH_ALIGNMENT,
            ID3D12CommandAllocator,
            ID3D12CommandQueue,
            ID3D12DescriptorHeap,
            ID3D12Device,
            ID3D12Fence,
            ID3D12GraphicsCommandList,
            ID3D12Resource
        }
    },
    System::Threading::{
        CreateEventA,
        INFINITE,
        WaitForSingleObject
    }
};
pub const FONT_TEX_ID: usize = usize::MAX;

#[allow(dead_code)]
#[derive(Debug)]
pub struct FontObjects {
    upload_buffer: ID3D12Resource,
    texture: ID3D12Resource
}
impl FontObjects {
    pub unsafe fn new(
        fonts: &mut FontAtlas,
        desc_heap: &ID3D12DescriptorHeap,
        device: &ID3D12Device,
        command_queue: &ID3D12CommandQueue
    ) -> windows::core::Result<Self> {
        let font_tex_cpu_desc_handle = desc_heap.GetCPUDescriptorHandleForHeapStart();
        let font_tex_gpu_desc_handle = desc_heap.GetGPUDescriptorHandleForHeapStart();
        // Build texture atlas and upload to graphics system
        let fa_tex = fonts.build_rgba32_texture();
        let mut props = D3D12_HEAP_PROPERTIES::default();
        props.Type = D3D12_HEAP_TYPE_DEFAULT;
        props.CPUPageProperty = D3D12_CPU_PAGE_PROPERTY_UNKNOWN;
        props.MemoryPoolPreference = D3D12_MEMORY_POOL_UNKNOWN;

        let mut desc = D3D12_RESOURCE_DESC::default();
        desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
        desc.Alignment = 0;
        desc.Width = fa_tex.width as u64;
        desc.Height = fa_tex.height;
        desc.DepthOrArraySize = 1;
        desc.MipLevels = 1;
        desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
        desc.SampleDesc.Count = 1;
        desc.SampleDesc.Quality = 0;
        desc.Layout = D3D12_TEXTURE_LAYOUT_UNKNOWN;
        desc.Flags = D3D12_RESOURCE_FLAG_NONE;

        let mut texture: Option<ID3D12Resource> = None;
        device.CreateCommittedResource::<ID3D12Resource>(
            &raw const props, 
            D3D12_HEAP_FLAG_NONE, 
            &raw const desc, 
            D3D12_RESOURCE_STATE_COPY_DEST, 
            None, &raw mut texture)?;

        let upload_pitch = (fa_tex.width * 4 + D3D12_TEXTURE_DATA_PITCH_ALIGNMENT - 1) 
            & !(D3D12_TEXTURE_DATA_PITCH_ALIGNMENT - 1);
        let upload_size = fa_tex.height * upload_pitch;
        desc.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
        desc.Alignment = 0;
        desc.Width = upload_size as u64;
        desc.Height = 1;
        desc.DepthOrArraySize = 1;
        desc.MipLevels = 1;
        desc.Format = DXGI_FORMAT_UNKNOWN;
        desc.SampleDesc.Count = 1;
        desc.SampleDesc.Quality = 0;
        desc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
        desc.Flags = D3D12_RESOURCE_FLAG_NONE;

        props.Type = D3D12_HEAP_TYPE_UPLOAD;
        props.CPUPageProperty = D3D12_CPU_PAGE_PROPERTY_UNKNOWN;
        props.MemoryPoolPreference = D3D12_MEMORY_POOL_UNKNOWN;

        let mut upload_buffer: Option<ID3D12Resource> = None;
        device.CreateCommittedResource::<ID3D12Resource>(
            &raw const props, 
            D3D12_HEAP_FLAG_NONE, 
            &raw const desc, 
            D3D12_RESOURCE_STATE_GENERIC_READ, 
            None, &raw mut upload_buffer)?;

        let mut mapped: *mut c_void = std::ptr::null_mut();
        let range = D3D12_RANGE {
            Begin: 0, End: upload_size as usize
        };
        upload_buffer.as_mut().unwrap().Map(0, Some(&raw const range), Some(&raw mut mapped))?;
        for y in 0..fa_tex.height as usize {
            std::ptr::copy_nonoverlapping(
                fa_tex.data.as_ptr().add(y * fa_tex.width as usize * 4), 
                (mapped as *mut u8).add(y * upload_pitch as usize),
               fa_tex.width as usize * 4 
            );
        }
        upload_buffer.as_mut().unwrap().Unmap(0, Some(&raw const range));

        let mut src_location = D3D12_TEXTURE_COPY_LOCATION::default();
        let mut dst_location = D3D12_TEXTURE_COPY_LOCATION::default();

        src_location.pResource = ManuallyDrop::new(upload_buffer.clone());
        src_location.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
        src_location.Anonymous.PlacedFootprint.Footprint.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
        src_location.Anonymous.PlacedFootprint.Footprint.Width = fa_tex.width;
        src_location.Anonymous.PlacedFootprint.Footprint.Height = fa_tex.height;
        src_location.Anonymous.PlacedFootprint.Footprint.Depth = 1;
        src_location.Anonymous.PlacedFootprint.Footprint.RowPitch = upload_pitch;

        dst_location.pResource = ManuallyDrop::new(texture.clone());
        dst_location.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        dst_location.Anonymous.SubresourceIndex = 0;

        let barrier = D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
            Anonymous: D3D12_RESOURCE_BARRIER_0 {
                Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                    pResource: ManuallyDrop::new(texture.clone()),
                    Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                    StateBefore: D3D12_RESOURCE_STATE_COPY_DEST,
                    StateAfter: D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE
                })
            }
        };

        let fence = device.CreateFence::<ID3D12Fence>(0, D3D12_FENCE_FLAG_NONE)?;
        let event = CreateEventA(None, false, false, None)?;
        let cmd_alloc = device.CreateCommandAllocator::<ID3D12CommandAllocator>(D3D12_COMMAND_LIST_TYPE_DIRECT)?;
        let cmd_list = device.CreateCommandList::<_, _, ID3D12GraphicsCommandList>(0, D3D12_COMMAND_LIST_TYPE_DIRECT, Some(&cmd_alloc), None)?;
        cmd_list.CopyTextureRegion(&raw const dst_location, 0, 0, 0, &raw const src_location, None);
        cmd_list.ResourceBarrier(&[barrier]);
        cmd_list.Close()?;
        command_queue.ExecuteCommandLists(&[Some(cmd_list.clone().into())]);
        command_queue.Signal(Some(&fence), 1)?;
        fence.SetEventOnCompletion(1, event)?;
        WaitForSingleObject(event, INFINITE);

        CloseHandle(event)?;

        let mut srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC::default();
        srv_desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
        srv_desc.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
        srv_desc.Anonymous.Texture2D.MipLevels = desc.MipLevels as u32;
        srv_desc.Anonymous.Texture2D.MostDetailedMip = 0;
        srv_desc.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
        device.CreateShaderResourceView(texture.as_ref(), Some(&srv_desc), font_tex_cpu_desc_handle);
        // Store our identifier
        fonts.tex_id = TextureId::new(font_tex_gpu_desc_handle.ptr as usize);
        Ok(Self { upload_buffer: upload_buffer.unwrap(), texture: texture.unwrap() })
    }
}