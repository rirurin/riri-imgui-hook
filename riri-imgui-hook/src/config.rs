use crate::registry::RegistryEntry;
use imgui::{
    Context as ImContext,
    FontConfig,
    FontGlyphRanges,
    FontSource
};
use riri_mod_tools_rt::mod_loader_data;
use riri_mod_tools_rt::logln;
use std::path::PathBuf;

pub fn imgui_common_init(imgui: &mut ImContext, registry: &RegistryEntry) {
    let mod_dir: String = mod_loader_data::get_directory_for_mod().into();
    let mod_dir = PathBuf::from(mod_dir);
    imgui.set_ini_filename(mod_dir.join("imgui.ini"));
    imgui.set_log_filename(None);
    // Set per-app flags
    imgui.io_mut().config_flags |= registry.get_config_flags_to_set();
    let font_path = mod_dir.join("NotoSansCJKjp-Medium.otf");
    let font_data = match std::fs::read(font_path) {
        Ok(f) => f,
        Err(_) => {
            logln!(Warning, "Custom font is missing! Falling back to default font");
            return;
        }
    };
    let mut font_config = FontConfig::default();
    font_config.glyph_ranges = FontGlyphRanges::japanese();
    imgui.fonts().add_font(&[FontSource::TtfData { data: font_data.as_slice(), size_pixels: 15., config: Some(font_config) }]);
}