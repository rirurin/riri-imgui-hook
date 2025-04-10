use riri_mod_tools_rt::address::ProcessInfo;
use windows::Win32::Foundation::HMODULE;

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

#[derive(Debug, Clone, Copy)]
pub enum RendererType {
    Direct3D11,
    Direct3D12
}

pub(crate) static REGISTRY_BY_EXE_NAME: &'static [(&'static str, RendererType)] = &[
    ("METAPHOR.exe", RendererType::Direct3D11),
    ("P5R.exe", RendererType::Direct3D11),
    ("P3R.exe", RendererType::Direct3D12),
    ("SMT5V-Win64-Shipping.exe", RendererType::Direct3D12),
];

pub fn get_target_renderer() -> RendererType {
    let process = ProcessInfo::get_current_process().unwrap();
    let name = process.get_executable_name();
    match REGISTRY_BY_EXE_NAME.iter().find(|p| p.0 == &name) {
        Some(v) => v.1,
        // Default to D3D11 for now
        None => RendererType::Direct3D11
    }
}