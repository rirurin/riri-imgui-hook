use proc_macro::TokenStream;

#[proc_macro]
pub fn create_static(input: TokenStream) -> TokenStream {
    riri_imgui_hook_proc_impl::create_static::create_static(input.into()).into()
}