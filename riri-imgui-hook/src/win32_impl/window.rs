use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{ HINSTANCE, HWND, LPARAM, LRESULT, WPARAM },
        System::LibraryLoader::GetModuleHandleA,
        UI::WindowsAndMessaging::{
            CreateWindowExW,
            DestroyWindow,
            DefWindowProcW,
            RegisterClassW,
            WNDCLASSW,
            WINDOW_EX_STYLE,
            WINDOW_STYLE,
            UnregisterClassW
        }
    }
};

const DUMMY_WINDOW_NAME: &'static str = "Blank\0";

pub struct DummyWindow {
    class_name: Vec<u16>,
    handle: HWND,
    instance: HINSTANCE,
}
impl DummyWindow {
    pub unsafe fn new() -> windows::core::Result<Self> {
        let mut class = WNDCLASSW::default();
        let class_name: Vec<u16> = DUMMY_WINDOW_NAME.encode_utf16().collect();
        let instance = GetModuleHandleA(None)?.into();
        class.lpfnWndProc = Some(Self::window_proc);
        class.hInstance = instance;
        class.lpszClassName = PCWSTR(class_name.as_ptr());
        RegisterClassW(&raw const class);
        let handle = CreateWindowExW(WINDOW_EX_STYLE(0), 
            PCWSTR(class_name.as_ptr()), PCWSTR(class_name.as_ptr()), WINDOW_STYLE(0), 
            0, 0, 128, 128, None, None, Some(instance), None)?;
        Ok(DummyWindow { class_name, handle, instance })
    }

    pub unsafe extern "system" fn window_proc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        DefWindowProcW(hwnd, umsg, wparam, lparam)
    }

    pub fn get_handle(&self) -> HWND {
        self.handle
    }
}

impl Drop for DummyWindow {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.handle).unwrap();
            UnregisterClassW(PCWSTR(self.class_name.as_ptr()), Some(self.instance)).unwrap();
        }
    }
}