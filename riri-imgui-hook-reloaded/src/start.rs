use crate::d3d11::D3D11Hook;
use riri_mod_tools_proc::{ create_hook, riri_init_fn };
use riri_mod_tools_rt::logln;
use std::{
    mem::MaybeUninit,
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
pub(crate) static BACKEND: Mutex<MaybeUninit<D3D11Hook>> = Mutex::new(MaybeUninit::uninit());

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
            unsafe { for dll in crate::d3d11::DLL_NAMES {
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
        let backend = D3D11Hook::new().unwrap();

        let present_ptr = backend.get_present_ptr() as usize;
        logln!(Verbose, "IDXGISwapChain::Present: 0x{:x}", present_ptr);
        create_hook!(present_ptr, crate::d3d11::hook_present);
        
        // let resize_buffers_ptr = backend.get_resize_buffers_ptr() as usize;
        // logln!(Verbose, "IDXGISwapChain::ResizeBuffers: 0x{:x}", resize_buffers_ptr);
        // create_hook!(resize_buffers_ptr, crate::d3d11::hook_resize_buffers);
        let mut backend_glb = BACKEND.try_lock().unwrap();
        *backend_glb = MaybeUninit::new(backend);
    });
}