// Adapted from imgui_impl_win32.cpp
// https://github.com/ocornut/imgui/blob/master/backends/imgui_impl_win32.cpp

use imgui::{
    BackendFlags,
    Context as ImContext,
    
};
use riri_mod_tools_rt::logln;
use std::{
    mem::MaybeUninit,
    time::{ Duration, Instant }
};
use windows::Win32::{
    Foundation::HWND,
    Graphics::Gdi::ScreenToClient,
    UI::WindowsAndMessaging::{
        GetClientRect,
        GetCursorPos,
        GetForegroundWindow,
    }
};

#[derive(Debug)]
pub struct Win32Impl {
    last_frame: Instant
}

pub(crate) static XINPUT_DLL: [&'static str; 5] = [
    "xinput1_4.dll\0", // Windows 8+
    "xinput1_3.dll\0", // DirectX SDK
    "xinput9_1_0.dll\0", // Windows Vista/Windows 7
    "xinput1_2.dll\0", // DirectX SDK
    "xinput1_1.dll\0" // DirectX SDK
];

impl Win32Impl {
    pub fn new(hwnd: HWND, ctx: &mut ImContext) -> Self {
        let platform_name = format!("riri-imgui-hook-win32");
        ctx.set_platform_name(Some(platform_name));
        // let viewport = ctx.main_viewport_mut();
        // viewport.platform_handle = hwnd.0;
        let io = ctx.io_mut();
        io.backend_flags.insert(BackendFlags::HAS_MOUSE_CURSORS);
        // io.backend_flags.insert(BackendFlags::HAS_SET_MOUSE_POS);
        Self {
            last_frame: Instant::now()
        }
    }

    pub fn new_frame(&mut self, hwnd: HWND, ctx: &mut ImContext) {
        let io = ctx.io_mut();
        // Set display size
        let mut rect = MaybeUninit::uninit();
        unsafe { GetClientRect(hwnd, rect.as_mut_ptr()).unwrap() };
        let rect = unsafe { rect.assume_init() };
        io.display_size = [(rect.right - rect.left) as f32, (rect.bottom - rect.top) as f32];

        // Set time
        let new_time = Instant::now();
        io.delta_time = new_time.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = new_time;

        self.update_mouse_pos(hwnd, ctx);
        // TODO: Workarounds for known Windows key handling issues
    }

    fn update_mouse_pos(&mut self, hwnd: HWND, ctx: &mut ImContext) {
        let focus_hwnd = unsafe { GetForegroundWindow() };
        if focus_hwnd == hwnd {
            let io = ctx.io_mut();
            let mut point = MaybeUninit::uninit();
            unsafe {
                GetCursorPos(point.as_mut_ptr()).unwrap();
                if ScreenToClient(hwnd, point.as_mut_ptr()).into() {
                    let point = point.assume_init();
                    let point_pos = [point.x as f32, point.y as f32];
                    io.add_mouse_pos_event(point_pos);
                }
            }
        }
    }
}