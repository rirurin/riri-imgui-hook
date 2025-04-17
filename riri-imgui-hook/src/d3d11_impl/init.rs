use crate::{
    registry::ModuleWrapper,
    win32_impl::window::DummyWindow
};
use riri_mod_tools_rt::logln;
use std::{
    mem::MaybeUninit,
    sync::OnceLock,
    time::Duration
};
use windows::{
    core::{ Error as WinError, HRESULT, PCSTR },
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
        },
        System::LibraryLoader,
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

static DIRECT3D_DLL: OnceLock<ModuleWrapper> = OnceLock::new();
static DIRECT3D_DLL_NAME: OnceLock<&'static str> = OnceLock::new();

pub unsafe fn start_d3d11() {
    for i in 0..20 {
        unsafe { for dll in crate::d3d11_impl::state::DLL_NAMES {
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
    let name = &name[..name.len()-1]; // remove null terminator
    logln!(Information, "Found DLL for {}: 0x{:x}", name, dll.0 as usize);
}