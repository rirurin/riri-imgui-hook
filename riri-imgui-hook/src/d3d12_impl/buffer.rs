use imgui::{ DrawIdx, DrawVert };
use std::marker::PhantomData;
use windows::Win32::Graphics::{
    Dxgi::Common::DXGI_FORMAT_UNKNOWN,
    Direct3D12::{
        D3D12_HEAP_FLAG_NONE,
        D3D12_HEAP_PROPERTIES,
        D3D12_HEAP_TYPE_UPLOAD,
        D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
        D3D12_MEMORY_POOL_UNKNOWN,
        D3D12_RESOURCE_DESC,
        D3D12_RESOURCE_DIMENSION_BUFFER,
        D3D12_RESOURCE_STATE_GENERIC_READ,
        D3D12_RESOURCE_FLAG_NONE,
        D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        ID3D12Device,
        ID3D12Resource,
    }
};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Buffer<T, const C: usize> 
where T: Sized 
{
    resource: ID3D12Resource,
    size: usize,
    _type: PhantomData<T>
}
impl<T, const C: usize> Buffer<T, C> 
where T: Sized
{
    pub unsafe fn new(device: &ID3D12Device, min_count: usize) -> windows::core::Result<Self> {
        let size = min_count + C;
        let mut props = D3D12_HEAP_PROPERTIES::default();
        props.Type = D3D12_HEAP_TYPE_UPLOAD;
        props.CPUPageProperty = D3D12_CPU_PAGE_PROPERTY_UNKNOWN;
        props.MemoryPoolPreference = D3D12_MEMORY_POOL_UNKNOWN;
        let mut desc = D3D12_RESOURCE_DESC::default();
        desc.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
        desc.Width = (size * size_of::<T>()) as u64;
        desc.Height = 1;
        desc.DepthOrArraySize = 1;
        desc.MipLevels = 1;
        desc.Format = DXGI_FORMAT_UNKNOWN;
        desc.SampleDesc.Count = 1;
        desc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
        desc.Flags = D3D12_RESOURCE_FLAG_NONE;
        let mut resource = None;
        device.CreateCommittedResource(
            &raw const props, 
            D3D12_HEAP_FLAG_NONE, 
            &raw const desc, 
            D3D12_RESOURCE_STATE_GENERIC_READ, 
            None, 
            &raw mut resource)?;
        Ok(Self { resource: resource.unwrap(), size, _type: PhantomData::<T> })
    }

    pub fn len(&self) -> usize { self.size }
    pub fn get_resource(&self) -> &ID3D12Resource {
        &self.resource
    }
}

const VERTEX_BUF_ADD_CAPACITY: usize = 5000;
const INDEX_BUF_ADD_CAPACITY: usize = 10000;

pub(crate) type VertexBuffer = Buffer<DrawVert, VERTEX_BUF_ADD_CAPACITY>;
pub(crate) type IndexBuffer = Buffer<DrawIdx, INDEX_BUF_ADD_CAPACITY>;