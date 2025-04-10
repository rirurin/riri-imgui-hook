use riri_imgui_hook::{
    d3d11_impl::{
        init::D3D11Init,
        state::D3D11Hook
    },
    win32_impl::state::Win32Impl
};
use imgui::{
    Context as ImContext,
    DrawData
};
use std::{
    error::Error,
    ffi::c_void,
    sync::Mutex
};
use riri_mod_tools_proc::{ create_hook, riri_hook_fn };
use riri_mod_tools_rt::logln;
use windows::Win32::{
    Foundation::{ HWND, LPARAM, LRESULT, WPARAM },
    Graphics::Dxgi::IDXGISwapChain
};

#[derive(Debug)]
pub enum Renderer {
    Direct3D11(D3D11Hook)
}
impl Renderer {
    pub fn render(&mut self, draw_data: &DrawData) -> windows::core::Result<()> {
        match self {
            Self::Direct3D11(r) => r.render(draw_data)
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Backend {
    imgui: ImContext,
    platform: Win32Impl,
    renderer: Renderer
}

impl Backend {
    pub unsafe fn make_hooks() {
        let dummy = D3D11Init::new().unwrap();
        let present_ptr = dummy.get_present_ptr() as usize;
        let resize_buffers_ptr = dummy.get_resize_buffers_ptr() as usize;
        logln!(Verbose, "IDXGISwapChain::Present: 0x{:x}", present_ptr);
        create_hook!(present_ptr, hook_present);
        logln!(Verbose, "IDXGISwapChain::ResizeBuffers: 0x{:x}", resize_buffers_ptr);
    }
    pub fn init(swapchain: IDXGISwapChain) -> Result<Self, Box<dyn Error>> {
        let desc = unsafe { (&swapchain).GetDesc()? };
        let swapchain_ptr = unsafe { *std::mem::transmute::<_, *const usize>(&swapchain) };
        logln!(Verbose, "Got HWND: {}, swapchain: 0x{:x}", desc.OutputWindow.0 as usize, swapchain_ptr);
        let mut imgui = ImContext::create();
        imgui.set_ini_filename(None);
        imgui.set_log_filename(None);
        // ImGui_ImplWin32_Init
        let platform = Win32Impl::new(&mut imgui, desc.OutputWindow);
        let wnd_proc_ptr = platform.get_wnd_proc();
        logln!(Verbose, "Hook WindowProc: 0x{:x}", wnd_proc_ptr);
        create_hook!(wnd_proc_ptr, hook_window_proc);
        // ImGui_ImplDX11_Init
        let renderer = Renderer::Direct3D11(unsafe { D3D11Hook::new(&mut imgui, swapchain)? });
        logln!(Verbose, "Platform: {}, Renderer: {}", imgui.platform_name().unwrap(), imgui.renderer_name().unwrap());
        Ok(Self { imgui, platform, renderer })
    }

    pub fn tick(&mut self) {
        self.platform.new_frame(&mut self.imgui);
        // self.renderer.new_frame (just calls CreateDeviceObjects if font sampler isn't initialized)
        let ui = self.imgui.new_frame();
        let mut opened = true;
        ui.show_demo_window(&mut opened);
        let draw_data = self.imgui.render();
        if let Err(e) = self.renderer.render(draw_data) {
            logln!(Error, "Error while rendering: {}", e);
        }
    }
}
unsafe impl Send for Backend {}
unsafe impl Sync for Backend {}

static DELAYED_HOOK_PRESENT: Mutex<usize> = Mutex::new(0);

#[riri_hook_fn(user_defined())]
pub unsafe extern "C" fn hook_present(p_swapchain: *const u8, sync_interval: u32, flags: u32) {
    let mut delayed_hook_lock = DELAYED_HOOK_PRESENT.lock().unwrap();
    if *delayed_hook_lock < 30 {
        *delayed_hook_lock += 1;
    } else {
        let swapchain = std::mem::transmute::<_, IDXGISwapChain>(p_swapchain).clone();
        let mut backend_lock = crate::start::BACKEND.lock().unwrap();
        match (*backend_lock).as_mut() {
            Some(v) => { v.tick(); },
            None => { *backend_lock = Some(Backend::init(swapchain).unwrap()); }
        }
    }
    original_function!(p_swapchain, sync_interval, flags)
}

#[riri_hook_fn(user_defined())]
pub unsafe extern "C" fn hook_window_proc(hook_hwnd: usize, umsg: u32, hook_wparam: usize, hook_lparam: isize) -> isize {
    let wparam = WPARAM(hook_wparam);
    let lparam = LPARAM(hook_lparam);
    let mut backend_lock = crate::start::BACKEND.lock().unwrap();
    let backend = (*backend_lock).as_mut().unwrap();
    match backend.platform.wnd_proc(&mut backend.imgui, umsg, wparam, lparam) {
        Some(r) => r.0,
        None => original_function!(hook_hwnd, umsg, hook_wparam, hook_lparam)
    }
}

/* 
#[riri_hook_fn(user_defined())]
pub unsafe extern "C" fn hook_resize_buffers(p_swapchain: *const u8, buffer_count: u32, 
    width: u32, height: u32, new_format: u32, swapchain_flags: u32) {
    logln!(Verbose, "Test hook resize buffers!");
    original_function!(p_swapchain, buffer_count, width, height, new_format, swapchain_flags)
}
*/