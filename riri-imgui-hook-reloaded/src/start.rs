use crate::backend::Backend;
use riri_mod_tools_proc::riri_init_fn;
use riri_mod_tools_rt::logln;
use std::{
    sync::{ Mutex, OnceLock },
    time::Duration
};
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader
    }
};

static DIRECT3D_DLL: OnceLock<ModuleWrapper> = OnceLock::new();
static DIRECT3D_DLL_NAME: OnceLock<&'static str> = OnceLock::new();
pub(crate) static BACKEND: Mutex<Option<Backend>> = Mutex::new(None);

#[derive(Debug)]
pub struct ModuleWrapper(HMODULE);
impl ModuleWrapper {
    pub fn get(&self) -> HMODULE { self.0 }
}
impl From<HMODULE> for ModuleWrapper {
    fn from(value: HMODULE) -> Self { Self(value) }
}
unsafe impl Sync for ModuleWrapper {}
unsafe impl Send for ModuleWrapper {}

#[riri_init_fn()]
fn start() {
    std::thread::spawn(|| {
        for i in 0..20 {
            unsafe { for dll in riri_imgui_hook::d3d11_impl::state::DLL_NAMES {
                let found = LibraryLoader::GetModuleHandleA(PCSTR(dll.as_ptr())).unwrap();
                if !found.is_invalid() {
                    let _ = DIRECT3D_DLL.set(found.into());
                    let _ = DIRECT3D_DLL_NAME.set(dll);
                    break;
                }
            }}
            if DIRECT3D_DLL.get().is_some() {
                break;
            } else {
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
        unsafe { Backend::make_hooks() }
    });
}