use riri_imgui_hook::{
    d3d11_impl::{
        init::D3D11Init,
        state::D3D11Hook
    },
    d3d12_impl::{
        init::D3D12Init,
        state::D3D12Hook
    },
    registry::{ RendererType, RegistryFlags },
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
    pub fn invalidate_render_target_view(&mut self, ctx: &mut ImContext) -> windows::core::Result<()> {
        match self {
            Self::Direct3D11(r) => r.invalidate_render_target_view(ctx),
            Self::Direct3D12(r) => r.invalidate_device_objects(ctx),
        }
    }
    pub fn create_render_target_view(&mut self, ctx: &mut ImContext) -> windows::core::Result<()> {
        match self {
            Self::Direct3D11(r) => unsafe { r.create_render_target_view(ctx) },
            Self::Direct3D12(r) => unsafe { r.create_device_objects(ctx) }
        }
    }
}

type CallbackTypeSignature = unsafe extern "C" fn(*mut ImUI, *mut <ImContext as RawWrapper>::Raw);
type CallbackInitAllocator = unsafe extern "C" fn(
    imgui::sys::ImGuiMemAllocFunc,
    imgui::sys::ImGuiMemFreeFunc,
    *mut std::ffi::c_void
);

#[allow(dead_code)]
#[derive(Debug)]
pub struct Backend {
    imgui: ImContext,
    platform: Win32Impl,
    renderer: Renderer,
    callbacks: HashSet<CallbackTypeSignature>,
    allocator_callbacks: Vec<CallbackInitAllocator>
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

        let present_ptr_thunk = riri_mod_tools_rt::sigscan_resolver::get_address_may_thunk_absolute(present_ptr).unwrap();
        let present_ptr_thunk = present_ptr_thunk.as_ptr() as usize;
        logln!(Verbose, "IDXGISwapChain::Present: 0x{:x} -> 0x{:x}", present_ptr, present_ptr_thunk);
        create_hook!(present_ptr_thunk, hook_present);
        let resize_buffers_ptr_thunk = riri_mod_tools_rt::sigscan_resolver::get_address_may_thunk_absolute(resize_buffers_ptr).unwrap();
        let resize_buffers_ptr_thunk = resize_buffers_ptr_thunk.as_ptr() as usize;
        logln!(Verbose, "IDXGISwapChain::ResizeBuffers: 0x{:x} -> 0x{:x}", resize_buffers_ptr, resize_buffers_ptr_thunk);
        create_hook!(resize_buffers_ptr_thunk, hook_resize_buffers);
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

    pub fn init_d3d11(swapchain: IDXGISwapChain, flags: RegistryFlags) -> Result<Self, Box<dyn Error>> {
        let desc = unsafe { (&swapchain).GetDesc()? };
        let swapchain_ptr = unsafe { *std::mem::transmute::<_, *const usize>(&swapchain) };
        logln!(Verbose, "Got HWND: {}, swapchain: 0x{:x}", desc.OutputWindow.0 as usize, swapchain_ptr);
        let mut imgui = ImContext::create();
        riri_imgui_hook::config::imgui_common_init(&mut imgui, *crate::start::TARGET.get().unwrap());

        // ImGui_ImplWin32_Init
        let platform = Win32Impl::new(&mut imgui, desc.OutputWindow);
        let wnd_proc_ptr = platform.get_wnd_proc();
        logln!(Verbose, "Hook WindowProc: 0x{:x}", wnd_proc_ptr);
        create_hook!(wnd_proc_ptr, hook_window_proc);
        // ImGui_ImplDX11_Init
        let renderer = Renderer::Direct3D11(D3D11Hook::new(&mut imgui, swapchain, flags)?);
        logln!(Verbose, "Platform: {}, Renderer: {}", imgui.platform_name().unwrap(), imgui.renderer_name().unwrap());
        Ok(Self { imgui, platform, renderer, callbacks: HashSet::new(), allocator_callbacks: vec![] })
    }

    pub fn init_d3d12(swapchain: IDXGISwapChain1, command_queue: ID3D12CommandQueue) -> Result<Self, Box<dyn Error>> {
        let desc = unsafe { (&swapchain).GetDesc()? };
        let swapchain_ptr = unsafe { *std::mem::transmute::<_, *const usize>(&swapchain) };
        logln!(Verbose, "Got HWND: {}, swapchain: 0x{:x}", desc.OutputWindow.0 as usize, swapchain_ptr);
        let mut imgui = ImContext::create();
        riri_imgui_hook::config::imgui_common_init(&mut imgui, *crate::start::TARGET.get().unwrap());

        // ImGui_ImplWin32_Init
        let platform = Win32Impl::new(&mut imgui, desc.OutputWindow);
        let wnd_proc_ptr = platform.get_wnd_proc();
        logln!(Verbose, "Hook WindowProc: 0x{:x}", wnd_proc_ptr);
        create_hook!(wnd_proc_ptr, hook_window_proc);
        // ImGui_ImplDX12_Init
        let renderer = Renderer::Direct3D12(unsafe { D3D12Hook::new(&mut imgui, swapchain, command_queue)? });
        logln!(Verbose, "Platform: {}, Renderer: {}", imgui.platform_name().unwrap(), imgui.renderer_name().unwrap());
        Ok(Self { imgui, platform, renderer, callbacks: HashSet::new(), allocator_callbacks: vec![] })
    }

    pub fn tick(&mut self) {
        self.platform.new_frame(&mut self.imgui);
        // self.renderer.new_frame (just calls CreateDeviceObjects if font sampler isn't initialized)
        // let _ui = self.imgui.new_frame();
        let ui = self.imgui.new_frame();
        let ui_ptr = &raw mut *ui;
        let ctx_ptr = unsafe { &raw mut *self.imgui.raw_mut() };
        if self.allocator_callbacks.len() > 0 {
            let (
                alloc, 
                free, 
                user
            ) = ImContext::get_allocator_functions();
            while let Some(cb) = self.allocator_callbacks.pop() {
                unsafe { cb(alloc, free, user) }
            }
        }
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
            let target = *crate::start::TARGET.get().unwrap();
            *backend_lock = match target.get_renderer() {
                RendererType::Direct3D11 => {
                    let swapchain = std::mem::transmute::<_, IDXGISwapChain>(p_swapchain).clone();
                    Some(Backend::init_d3d11(swapchain, target.get_flags()).unwrap())
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

#[riri_hook_fn(user_defined())]
pub unsafe extern "C" fn hook_resize_buffers(p_swapchain: *const u8, buffer_count: u32, 
    width: u32, height: u32, new_format: u32, swapchain_flags: u32) -> i32 {
    let mut backend_lock = crate::start::BACKEND.lock().unwrap();
    if let Some(b) = (*backend_lock).as_mut() {
        let _ = b.renderer.invalidate_render_target_view(&mut b.imgui);
    }
    drop(backend_lock);
    let hresult = original_function!(p_swapchain, buffer_count, width, height, new_format, swapchain_flags);
    let mut backend_lock = crate::start::BACKEND.lock().unwrap();
    if let Some(b) = (*backend_lock).as_mut() {
        let _ = b.renderer.create_render_target_view(&mut b.imgui);
    }
    drop(backend_lock);
    hresult
}

#[no_mangle]
pub unsafe extern "C" fn add_gui_callback(cb: unsafe extern "C" fn(*mut u8, *mut u8), version: *const i8) {
    let external_ver = std::ffi::CStr::from_ptr(version).to_str().unwrap();
    let local_ver = imgui::dear_imgui_version();
    if external_ver != local_ver {
        logln!(Error, "Imgui version is {}, but external crate uses version {}", local_ver, external_ver);
        return;
    }
    let mut backend_lock = crate::start::BACKEND.lock().unwrap();
    let backend = (*backend_lock).as_mut().unwrap();
    let cb = std::mem::transmute::<_, CallbackTypeSignature>(cb);
    backend.callbacks.insert(cb);
}

#[no_mangle]
pub unsafe extern "C" fn add_allocator(cb: unsafe extern "C" fn (*mut u8, *mut u8, *mut u8)) {
    let mut backend_lock = crate::start::BACKEND.lock().unwrap();
    let backend = (*backend_lock).as_mut().unwrap();
    let cb = std::mem::transmute::<_, CallbackInitAllocator>(cb);
    backend.allocator_callbacks.push(cb);
}

#[no_mangle]
pub unsafe extern "C" fn remove_gui_callback(cb: unsafe extern "C" fn(*mut u8, *mut u8)) {
    let mut backend_lock = crate::start::BACKEND.lock().unwrap();
    let backend = (*backend_lock).as_mut().unwrap();
    let cb = std::mem::transmute::<_, CallbackTypeSignature>(cb);
    backend.callbacks.remove(&cb);
}