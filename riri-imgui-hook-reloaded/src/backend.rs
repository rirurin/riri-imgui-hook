use riri_imgui_hook::{
    d3d11_impl::{
        init::D3D11Init,
        state::D3D11Hook
    },
    d3d12_impl::{
        init::D3D12Init,
        state::D3D12Hook
    },
    registry::RendererType,
    win32_impl::state::Win32Impl
};
use imgui::{
    internal::RawWrapper,
    Context as ImContext,
    DrawData,
    Ui as ImUI
};
use std::{
    collections::HashSet,
    error::Error,
    ptr::NonNull,
    sync::Mutex,
};
use riri_mod_tools_proc::{ create_hook, riri_hook_fn };
use riri_mod_tools_rt::logln;
use windows::Win32::{
    Foundation::{ LPARAM, WPARAM },
    Graphics::{
        Direct3D12::ID3D12CommandQueue,
        Dxgi::{ IDXGISwapChain, IDXGISwapChain1 }
    },
};

#[derive(Debug)]
pub enum Renderer {
    Direct3D11(D3D11Hook),
    Direct3D12(D3D12Hook),
}
impl Renderer {
    pub fn render(&mut self, draw_data: &DrawData) -> windows::core::Result<()> {
        match self {
            Self::Direct3D11(r) => r.render(draw_data),
            Self::Direct3D12(r) => r.render(draw_data)
        }
    }
}

type CallbackTypeSignature = unsafe extern "C" fn(*mut ImUI, *mut <ImContext as RawWrapper>::Raw);

#[allow(dead_code)]
#[derive(Debug)]
pub struct Backend {
    imgui: ImContext,
    platform: Win32Impl,
    renderer: Renderer,
    callbacks: HashSet<CallbackTypeSignature>
}

struct CommandQueueStore(Mutex<Option<NonNull<u8>>>);
impl CommandQueueStore {
    const fn new() -> Self {
        Self(Mutex::new(None))
    }
    fn try_get(&self) -> Option<*mut u8> {
        let val = self.0.lock().unwrap();
        (*val).as_ref().map(|v| v.as_ptr())
    }
    fn set(&self, val: *mut u8) {
        let mut lock = self.0.lock().unwrap();
        *lock = Some(unsafe { NonNull::new_unchecked(val) });
    }
}
unsafe impl Send for CommandQueueStore {}
unsafe impl Sync for CommandQueueStore {}

static COMMAND_QUEUE: CommandQueueStore = CommandQueueStore::new();

impl Backend {
    pub unsafe fn make_hooks_d3d11() {
        let dummy = match D3D11Init::new() {
            Ok(v) => v,
            Err(e) => {
                logln!(Error, "Error initializing D3D11: {}. Closing Imgui Hook.", e);
                return;
            }
        };
        let present_ptr = dummy.get_present_ptr() as usize;
        let resize_buffers_ptr = dummy.get_resize_buffers_ptr() as usize;
        logln!(Verbose, "IDXGISwapChain::Present: 0x{:x}", present_ptr);
        create_hook!(present_ptr, hook_present);
        logln!(Verbose, "IDXGISwapChain::ResizeBuffers: 0x{:x}", resize_buffers_ptr);
    }

    pub unsafe fn make_hooks_d3d12() {
        let dummy = match D3D12Init::new() {
            Ok(v) => v,
            Err(e) => {
                logln!(Error, "Error initializing D3D12: {}. Closing Imgui Hook.", e);
                return;
            }
        };
        let present_ptr = dummy.get_present_ptr() as usize;
        let resize_buffers_ptr = dummy.get_resize_buffers_ptr() as usize;
        let exec_cmd_list_ptr = dummy.get_execute_command_lists_ptr() as usize;
        logln!(Verbose, "IDXGISwapChain::Present: 0x{:x}", present_ptr);
        create_hook!(present_ptr, hook_present);
        logln!(Verbose, "IDXGISwapChain::ResizeBuffers: 0x{:x}", resize_buffers_ptr);
        logln!(Verbose, "ID3D12CommandQueue::ExecuteCommandLists: 0x{:x}", exec_cmd_list_ptr);
        create_hook!(exec_cmd_list_ptr, hook_execute_command_lists);
    }

    pub fn init_d3d11(swapchain: IDXGISwapChain) -> Result<Self, Box<dyn Error>> {
        let desc = unsafe { (&swapchain).GetDesc()? };
        let swapchain_ptr = unsafe { *std::mem::transmute::<_, *const usize>(&swapchain) };
        logln!(Verbose, "Got HWND: {}, swapchain: 0x{:x}", desc.OutputWindow.0 as usize, swapchain_ptr);
        let mut imgui = ImContext::create();

        imgui.set_ini_filename(None);
        imgui.set_log_filename(None);

        // Set per-app flags
        {
            let target = crate::start::TARGET.get().unwrap();
            imgui.io_mut().config_flags |= target.get_config_flags_to_set();
        }

        // ImGui_ImplWin32_Init
        let platform = Win32Impl::new(&mut imgui, desc.OutputWindow);
        let wnd_proc_ptr = platform.get_wnd_proc();
        logln!(Verbose, "Hook WindowProc: 0x{:x}", wnd_proc_ptr);
        create_hook!(wnd_proc_ptr, hook_window_proc);
        // ImGui_ImplDX11_Init
        let renderer = Renderer::Direct3D11(unsafe { D3D11Hook::new(&mut imgui, swapchain)? });
        logln!(Verbose, "Platform: {}, Renderer: {}", imgui.platform_name().unwrap(), imgui.renderer_name().unwrap());
        Ok(Self { imgui, platform, renderer, callbacks: HashSet::new() })
    }

    pub fn init_d3d12(swapchain: IDXGISwapChain1, command_queue: ID3D12CommandQueue) -> Result<Self, Box<dyn Error>> {
        let desc = unsafe { (&swapchain).GetDesc()? };
        let swapchain_ptr = unsafe { *std::mem::transmute::<_, *const usize>(&swapchain) };
        logln!(Verbose, "Got HWND: {}, swapchain: 0x{:x}", desc.OutputWindow.0 as usize, swapchain_ptr);
        let mut imgui = ImContext::create();

        imgui.set_ini_filename(None);
        imgui.set_log_filename(None);

        // Set per-app flags
        {
            let target = crate::start::TARGET.get().unwrap();
            imgui.io_mut().config_flags |= target.get_config_flags_to_set();
        }

        // ImGui_ImplWin32_Init
        let platform = Win32Impl::new(&mut imgui, desc.OutputWindow);
        let wnd_proc_ptr = platform.get_wnd_proc();
        logln!(Verbose, "Hook WindowProc: 0x{:x}", wnd_proc_ptr);
        create_hook!(wnd_proc_ptr, hook_window_proc);
        // ImGui_ImplDX12_Init
        let renderer = Renderer::Direct3D12(unsafe { D3D12Hook::new(&mut imgui, swapchain, command_queue)? });
        logln!(Verbose, "Platform: {}, Renderer: {}", imgui.platform_name().unwrap(), imgui.renderer_name().unwrap());
        Ok(Self { imgui, platform, renderer, callbacks: HashSet::new() })
    }

    pub fn tick(&mut self) {
        self.platform.new_frame(&mut self.imgui);
        // self.renderer.new_frame (just calls CreateDeviceObjects if font sampler isn't initialized)
        let ui = self.imgui.new_frame();
        let ui_ptr = &raw mut *ui;
        let ctx_ptr = unsafe { &raw mut *self.imgui.raw_mut() };
        for cb in self.callbacks.iter() {
            unsafe { cb(ui_ptr, ctx_ptr) }
        }
        let draw_data = self.imgui.render();
        if let Err(e) = self.renderer.render(draw_data) {
            logln!(Error, "Error while rendering: {}", e);
        }
    }
}
unsafe impl Send for Backend {}
unsafe impl Sync for Backend {}

#[riri_hook_fn(user_defined())]
pub unsafe extern "C" fn hook_present(p_swapchain: *const u8, sync_interval: u32, flags: u32) {
    let mut backend_lock = crate::start::BACKEND.lock().unwrap();
    match (*backend_lock).as_mut() {
        Some(v) => { v.tick(); },
        None => { 
            *backend_lock = match crate::start::TARGET.get().unwrap().get_renderer() {
                RendererType::Direct3D11 => {
                    let swapchain = std::mem::transmute::<_, IDXGISwapChain>(p_swapchain).clone();
                    Some(Backend::init_d3d11(swapchain).unwrap())
                },
                RendererType::Direct3D12 => {
                    if let Some(cmd) = COMMAND_QUEUE.try_get() {
                        let swapchain = std::mem::transmute::<_, IDXGISwapChain1>(p_swapchain).clone();
                        let cmd_queue = std::mem::transmute::<_, ID3D12CommandQueue>(cmd).clone();
                        Some(Backend::init_d3d12(swapchain, cmd_queue).unwrap())
                    } else { None }
                },
            } 
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
        None => {
            drop(backend_lock);
            original_function!(hook_hwnd, umsg, hook_wparam, hook_lparam)
        }
    }
}

#[riri_hook_fn(user_defined())]
pub unsafe extern "C" fn hook_execute_command_lists(
    p_command_queue: *mut u8, 
    command_lists_len: u32, 
    p_command_lists: *mut u8
    ) {
        if COMMAND_QUEUE.try_get().is_none() {
            logln!(Verbose, "ExecuteCommandLists set to 0x{:x}", p_command_queue as usize);
            COMMAND_QUEUE.set(p_command_queue);
        }
        original_function!(p_command_queue, command_lists_len, p_command_lists)
    }

/* 
#[riri_hook_fn(user_defined())]
pub unsafe extern "C" fn hook_resize_buffers(p_swapchain: *const u8, buffer_count: u32, 
    width: u32, height: u32, new_format: u32, swapchain_flags: u32) {
    logln!(Verbose, "Test hook resize buffers!");
    original_function!(p_swapchain, buffer_count, width, height, new_format, swapchain_flags)
}
*/

#[no_mangle]
pub unsafe extern "C" fn add_gui_callback(cb: unsafe extern "C" fn(*mut u8, *mut u8)) {
    let mut backend_lock = crate::start::BACKEND.lock().unwrap();
    let backend = (*backend_lock).as_mut().unwrap();
    let cb = std::mem::transmute::<_, CallbackTypeSignature>(cb);
    backend.callbacks.insert(cb);
}

#[no_mangle]
pub unsafe extern "C" fn remove_gui_callback(cb: unsafe extern "C" fn(*mut u8, *mut u8)) {
    let mut backend_lock = crate::start::BACKEND.lock().unwrap();
    let backend = (*backend_lock).as_mut().unwrap();
    let cb = std::mem::transmute::<_, CallbackTypeSignature>(cb);
    backend.callbacks.remove(&cb);
}