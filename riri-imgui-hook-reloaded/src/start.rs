use crate::backend::Backend;
use riri_imgui_hook::{
    d3d11_impl::init as d3d11_init,
    d3d12_impl::init as d3d12_init,
    registry::{ RendererType, RegistryEntry }
};
use riri_mod_tools_proc::riri_mods_loaded_fn;
use std::sync::{ Mutex, OnceLock };

pub(crate) static BACKEND: Mutex<Option<Backend>> = Mutex::new(None);
pub(crate) static TARGET: OnceLock<&'static RegistryEntry<'static>> = OnceLock::new();

// Wait for all other DLLs to load in case they decide to hook the same functions as us
// (e.g Steam hooks Present and ResizeBuffers for their game overlay)
// afaik Gamepass does not do this
#[riri_mods_loaded_fn()]
fn start() {
    let _ = TARGET.set(riri_imgui_hook::registry::get_registry_entry());
    let value = *TARGET.get().unwrap();
    match value.get_renderer() {
        RendererType::Direct3D11 => {
            std::thread::spawn(|| { unsafe { 
                d3d11_init::start_d3d11();
                Backend::make_hooks_d3d11();
            }});
        },
        RendererType::Direct3D12 => {
            std::thread::spawn(|| { unsafe { 
                d3d12_init::start_d3d12();
                Backend::make_hooks_d3d12();
            }});
        }
    }
}