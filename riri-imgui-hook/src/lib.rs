pub mod config;
pub mod d3d11_impl {
    pub mod backup;
    pub mod buffer;
    pub mod devices;
    pub mod font;
    pub mod init;
    pub mod shader;
    pub mod state;
}
pub mod d3d12_impl {
    pub mod buffer;
    pub mod font;
    pub mod init;
    pub mod pipeline;
    pub mod signature;
    pub mod state;
}
pub mod globals;
pub mod registry;
pub mod win32_impl {
    pub mod state;
    pub mod window;
}