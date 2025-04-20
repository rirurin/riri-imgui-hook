use bitflags::bitflags;
use imgui::ConfigFlags;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RendererType {
    Direct3D11,
    Direct3D12
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct RegistryFlags : u32 {
        const USE_SRGB = 1 << 0;
    }
}

pub struct RegistryEntry<'a> {
    executable: &'a str,
    renderer: RendererType,
    io_config_flags_set: ConfigFlags,
    flags: RegistryFlags
}
impl<'a> RegistryEntry<'a> {
    const fn new(
        executable: &'a str, 
        renderer: RendererType, 
        io_config_flags_set: ConfigFlags,
        flags: RegistryFlags
    ) -> Self {
        Self { executable, renderer, io_config_flags_set, flags }
    }

    pub fn get_renderer(&self) -> RendererType {
        self.renderer
    }
    pub fn get_config_flags_to_set(&self) -> ConfigFlags {
        self.io_config_flags_set
    }
    pub fn get_flags(&self) -> RegistryFlags {
        self.flags
    }
}

pub(crate) static REGISTRY_BY_EXE_NAME: &'static [RegistryEntry<'static>] = &[
    RegistryEntry::new("METAPHOR.exe", RendererType::Direct3D11, ConfigFlags::empty(), RegistryFlags::USE_SRGB),
    RegistryEntry::new("P5R.exe", RendererType::Direct3D11, ConfigFlags::empty(), RegistryFlags::empty()),
    RegistryEntry::new("P3R.exe", RendererType::Direct3D12, ConfigFlags::empty(), RegistryFlags::empty()),
    RegistryEntry::new("SMT5V-Win64-Shipping.exe", RendererType::Direct3D12, ConfigFlags::empty(), RegistryFlags::empty()),
];
pub(crate) static DEFAULT_REGISTRY: RegistryEntry<'static> = 
    RegistryEntry::new("P5R.exe", RendererType::Direct3D11, ConfigFlags::empty(), RegistryFlags::empty());

pub fn get_registry_entry() -> &'static RegistryEntry<'static> {
    let process = ProcessInfo::get_current_process().unwrap();
    let name = process.get_executable_name();
    match REGISTRY_BY_EXE_NAME.iter().find(|p| p.executable == &name) {
        Some(v) => v,
        None => &DEFAULT_REGISTRY
    }
}