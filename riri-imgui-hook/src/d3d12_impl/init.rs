// Adapted from D3D12Hook from original C# implementation of riri.imguihook
use crate::{
    registry::ModuleWrapper,
    win32_impl::window::DummyWindow
};
use riri_mod_tools_rt::logln;
use std::{
    sync::OnceLock,
    time::Duration
};
use windows::{
    core::{ Error as WinError, PCSTR },
    Win32::{
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D12::{
                D3D12_COMMAND_QUEUE_FLAG_NONE,
                D3D12_COMMAND_LIST_TYPE_DIRECT,
                D3D12_COMMAND_QUEUE_DESC,
                D3D12CreateDevice,
                ID3D12CommandQueue,
                ID3D12Device
            },
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_UNSPECIFIED,
                    DXGI_FORMAT_R8G8B8A8_UNORM,
                    DXGI_SAMPLE_DESC
                },
                CreateDXGIFactory2,
                DXGI_ADAPTER_FLAG_SOFTWARE,
                DXGI_CREATE_FACTORY_FLAGS,
                DXGI_ERROR_NOT_FOUND,
                DXGI_SCALING_NONE,
                DXGI_SWAP_CHAIN_DESC1,
                DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH,
                DXGI_SWAP_EFFECT_FLIP_DISCARD,
                DXGI_USAGE_RENDER_TARGET_OUTPUT,
                IDXGIAdapter1,
                IDXGIFactory4,
                IDXGISwapChain1
            },
        },
        System::LibraryLoader,
    },
};

// Create dummy objects for the purpose of extracting vtables from
#[allow(dead_code)]
#[derive(Debug)]
pub struct D3D12Init {
    factory: IDXGIFactory4,
    adapter: IDXGIAdapter1,
    device: ID3D12Device,
    command_queue: ID3D12CommandQueue,
    swapchain: IDXGISwapChain1,
}

impl D3D12Init {
    unsafe fn create_swapchain_desc() -> windows::core::Result<DXGI_SWAP_CHAIN_DESC1> {
        Ok(DXGI_SWAP_CHAIN_DESC1 {
            Width: 128, 
            Height: 128, 
            Format: DXGI_FORMAT_R8G8B8A8_UNORM, 
            Stereo: false.into(),
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_NONE,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode: DXGI_ALPHA_MODE_UNSPECIFIED,
            Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH.0 as u32
        })
    }

    unsafe fn get_device(adapter: &IDXGIAdapter1) -> windows::core::Result<ID3D12Device> {
        let mut device_out: Option<ID3D12Device> = None;
        D3D12CreateDevice(
            Some(adapter.into()), 
            D3D_FEATURE_LEVEL_11_0, 
            &raw mut device_out)
            .map(|_| device_out.unwrap())
    }

    unsafe fn get_adapter(factory: &IDXGIFactory4) -> windows::core::Result<IDXGIAdapter1> {
        let mut index = 0;
        while let Ok(a) = factory.EnumAdapters1(index) {
            index += 1;
            let desc = a.GetDesc1()?;
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 { continue; }
            if let Ok(_) = Self::get_device(&a) { return Ok(a) }
        }
        Err(WinError::from_hresult(DXGI_ERROR_NOT_FOUND))
    }

    unsafe fn get_command_queue(device: &ID3D12Device) -> windows::core::Result<ID3D12CommandQueue> {
        let desc = D3D12_COMMAND_QUEUE_DESC {
            Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
            Priority: 0,
            Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
            NodeMask: 0
        };
        device.CreateCommandQueue(&raw const desc)
    }

    pub unsafe fn new() -> windows::core::Result<Self> {
        let factory = CreateDXGIFactory2::<IDXGIFactory4>(DXGI_CREATE_FACTORY_FLAGS(0))?;
        let adapter = Self::get_adapter(&factory)?;
        let device = Self::get_device(&adapter)?;
        let command_queue = Self::get_command_queue(&device)?;
        let dummy_window = DummyWindow::new()?;
        let desc = Self::create_swapchain_desc()?;
        let swapchain = factory.CreateSwapChainForHwnd(
            Some((&command_queue).into()), 
            dummy_window.get_handle(), 
            &raw const desc, 
            None, 
            None)?;
        Ok(Self { factory, adapter, device, command_queue, swapchain })
    }
    pub unsafe fn get_present_ptr(&self) -> *const u8 {
        windows_core::Interface::vtable(&self.swapchain).base__.Present as *const u8
    }

    pub unsafe fn get_resize_buffers_ptr(&self) -> *const u8 {
        windows_core::Interface::vtable(&self.swapchain).base__.ResizeBuffers as *const u8
    }

    pub unsafe fn get_execute_command_lists_ptr(&self) -> *const u8 {
        windows_core::Interface::vtable(&self.command_queue).ExecuteCommandLists as *const u8
    }
}

static DIRECT3D_DLL: OnceLock<ModuleWrapper> = OnceLock::new();
static DIRECT3D_DLL_NAME: OnceLock<&'static str> = OnceLock::new();

pub unsafe fn start_d3d12() {
    for i in 0..20 {
        unsafe { for dll in crate::d3d12_impl::state::DLL_NAMES {
            if let Ok(h) = LibraryLoader::GetModuleHandleA(PCSTR(dll.as_ptr())) {
                if !h.is_invalid() {
                    let _ = DIRECT3D_DLL.set(h.into());
                    let _ = DIRECT3D_DLL_NAME.set(dll);
                    break;
                }
            }
        }}
        if DIRECT3D_DLL.get().is_some() {
            break;
        } else {
            // This is expected to be run on a separate thread spun up by riri-imgui-hook-reloaded
            std::thread::sleep(Duration::from_millis(250 + (100 * i * i)));
        }
    }
    if DIRECT3D_DLL.get().is_none() {
        logln!(Error, "Could not hook to Direct3D DLL. Closing Imgui Hook");
        return;
    }
    let dll = DIRECT3D_DLL.get().unwrap().get();
    let name = *DIRECT3D_DLL_NAME.get().unwrap();
    logln!(Information, "Found DLL for {}: 0x{:x}", name, dll.0 as usize);
}