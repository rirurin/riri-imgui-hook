use crate::{
    win32_impl::init::Win32Impl,
    window::DummyWindow
};
use imgui::{
    BackendFlags,
    Context as ImContext
};
use riri_mod_tools_proc::riri_hook_fn;
use riri_mod_tools_rt::logln;
use std::sync::Mutex;
use windows::Win32::{
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
            ID3D11DeviceContext
        }
    }
};

// Adapted from original C# implementation of riri-imgui-hook:
// https://github.com/rirurin/riri.imguihook/blob/master/riri.imguihook/D3D11Hook.cs

pub(crate) static DLL_NAMES: [&'static str; 5] = [
    "d3d11.dll\0",
    "d3d11_1.dll\0",
    "d3d11_2.dll\0",
    "d3d11_3.dll\0",
    "d3d11_4.dll\0"
];

#[derive(Debug)]
pub struct D3D11Hook {
    dummy_objects: D3D11DummyObjects,
    imgui: ImContext,
    platform: Option<Box<Win32Impl>>,
    hook_handle: HWND
}
unsafe impl Send for D3D11Hook {}
unsafe impl Sync for D3D11Hook {}

#[allow(dead_code)]
#[derive(Debug)]
pub struct D3D11DummyObjects {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    swapchain: IDXGISwapChain
}

impl D3D11Hook {
    pub fn new() -> windows::core::Result<Self> {
        let mut imgui = ImContext::create();
        imgui.set_ini_filename(None);
        imgui.set_renderer_name(Some(format!("riri-imgui-hook-d3d11")));
        {
            let io = imgui.io_mut();
            io.backend_flags.insert(BackendFlags::RENDERER_HAS_VTX_OFFSET);
        }
        Ok(Self {
            dummy_objects: unsafe { D3D11DummyObjects::new()? },
            imgui, platform: None, hook_handle: HWND::default()
        })
    }

    pub(crate) unsafe fn get_present_ptr(&self) -> *const u8 {
        windows_core::Interface::vtable(&self.dummy_objects.swapchain).Present as *const u8
    }

    // pub(crate) unsafe fn get_resize_buffers_ptr(&self) -> *const u8 {
    //     windows_core::Interface::vtable(&self.dummy_objects.swapchain).ResizeBuffers as *const u8
    // } 
}

static DELAYED_HOOK_PRESENT: Mutex<usize> = Mutex::new(0);

#[riri_hook_fn(user_defined())]
pub unsafe extern "C" fn hook_present(p_swapchain: *const u8, sync_interval: u32, flags: u32) {
    let mut delayed_hook_lock = DELAYED_HOOK_PRESENT.lock().unwrap();
    if *delayed_hook_lock < 30 {
        *delayed_hook_lock += 1;
    } else {
        let mut backend_lock = crate::start::BACKEND.lock().unwrap();
        let backend = backend_lock.assume_init_mut();
        let swapchain = std::mem::transmute::<_, IDXGISwapChain>(p_swapchain);
        let desc = unsafe { swapchain.GetDesc().unwrap() };
        if backend.hook_handle.is_invalid() {
            // Initialize Present
            backend.hook_handle = desc.OutputWindow;
            backend.platform = Some(Box::new(Win32Impl::new(backend.hook_handle, &mut backend.imgui)));
            logln!(Verbose, "Got HWND: {}, swapchain: 0x{:x}", desc.OutputWindow.0 as usize, p_swapchain as usize);
        } else {
            // Get last frame
            let plat = backend.platform.as_mut().unwrap().as_mut();
            plat.new_frame(backend.hook_handle, &mut backend.imgui);

        }
        // logln!(Verbose, "Test hook present! (swapchain: 0x{:x})", p_swapchain as usize);
    }
    original_function!(p_swapchain, sync_interval, flags)
}

/* 
#[riri_hook_fn(user_defined())]
pub unsafe extern "C" fn hook_resize_buffers(p_swapchain: *const u8, buffer_count: u32, 
    width: u32, height: u32, new_format: u32, swapchain_flags: u32) {
    logln!(Verbose, "Test hook resize buffers!");
    original_function!(p_swapchain, buffer_count, width, height, new_format, swapchain_flags)
}
*/

impl D3D11DummyObjects { 
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

    pub(crate) unsafe fn new() -> windows::core::Result<Self> {
        let feature_levels: [D3D_FEATURE_LEVEL; 1] = [D3D_FEATURE_LEVEL_11_0];
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        let mut swapchain: Option<IDXGISwapChain> = None;
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
            None, 
            Some(&raw mut context))?;
        Ok(Self {
            device: device.unwrap(),
            context: context.unwrap(),
            swapchain: swapchain.unwrap(),
        })
    }
}