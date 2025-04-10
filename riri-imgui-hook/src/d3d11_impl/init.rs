use crate::win32_impl::window::DummyWindow;
use std::mem::MaybeUninit;
use windows::{
    core::{ Error as WinError, HRESULT },
    Win32::{
        Foundation::{ HMODULE, HWND },
        Graphics::{
            Dxgi::{
                Common::{
                    DXGI_FORMAT_R8G8B8A8_UNORM,
                    DXGI_MODE_DESC,
                    DXGI_MODE_SCALING_UNSPECIFIED,
                    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                    DXGI_RATIONAL,
                    DXGI_SAMPLE_DESC,
                },
                DXGI_SWAP_CHAIN_DESC,
                DXGI_SWAP_EFFECT_DISCARD,
                DXGI_USAGE_RENDER_TARGET_OUTPUT,
                IDXGISwapChain
            },
            Direct3D::{
                D3D_DRIVER_TYPE_HARDWARE,
                D3D_FEATURE_LEVEL,
                D3D_FEATURE_LEVEL_11_0
            },
            Direct3D11::{
                D3D11CreateDeviceAndSwapChain,
                D3D11_CREATE_DEVICE_FLAG,
                D3D11_SDK_VERSION,
                ID3D11Device,
            }
        }
    }
};

// Create dummy objects for the purpose of extracting vtables from
#[derive(Debug)]
pub struct D3D11Init {
    swapchain: IDXGISwapChain,
}

impl D3D11Init {
    unsafe fn create_swapchain_desc(hwnd: HWND) -> windows::core::Result<DXGI_SWAP_CHAIN_DESC> {
        Ok(DXGI_SWAP_CHAIN_DESC {
            BufferDesc: DXGI_MODE_DESC { 
                Width: 128, 
                Height: 128, 
                RefreshRate: DXGI_RATIONAL { Numerator: 60, Denominator: 1 }, 
                Format: DXGI_FORMAT_R8G8B8A8_UNORM, 
                ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, 
                Scaling: DXGI_MODE_SCALING_UNSPECIFIED 
            },
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 1,
            OutputWindow: hwnd,
            Windowed: true.into(),
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            Flags: 0
        })
    }

    pub unsafe fn new() -> windows::core::Result<Self> {
        let feature_levels: [D3D_FEATURE_LEVEL; 1] = [D3D_FEATURE_LEVEL_11_0];
        let mut device: Option<ID3D11Device> = None;
        let mut swapchain: Option<IDXGISwapChain> = None;
        let mut feature_level: MaybeUninit<D3D_FEATURE_LEVEL> = MaybeUninit::uninit();
        let dummy_window = DummyWindow::new()?;
        let desc = Self::create_swapchain_desc(dummy_window.get_handle())?;
        D3D11CreateDeviceAndSwapChain(
            None, 
            D3D_DRIVER_TYPE_HARDWARE, 
            HMODULE::default(), 
            D3D11_CREATE_DEVICE_FLAG(0), 
            Some(feature_levels.as_slice()), 
            D3D11_SDK_VERSION, 
            Some(&raw const desc),
            Some(&raw mut swapchain), 
            Some(&raw mut device), 
            Some(feature_level.as_mut_ptr()), 
            None)?;
        if feature_level.assume_init() != windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0 {
            return Err(WinError::from_hresult(HRESULT::from_win32(0x80004005))) // E_FAIL
        }
        let swapchain = match swapchain {
            Some(v) => v,
            None => return Err(WinError::from_hresult(HRESULT::from_win32(0x80004005))) // E_FAIL
        };
        Ok(Self { swapchain })
    }

    pub unsafe fn get_present_ptr(&self) -> *const u8 {
        windows_core::Interface::vtable(&self.swapchain).Present as *const u8
    }

    pub unsafe fn get_resize_buffers_ptr(&self) -> *const u8 {
        windows_core::Interface::vtable(&self.swapchain).ResizeBuffers as *const u8
    } 
}