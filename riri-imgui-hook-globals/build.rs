use std::{
    error::Error,
    path::{ Path, PathBuf }
};
use riri_imgui_hook_proc_impl::create_static::create_static_links;
use proc_macro2::Span;
use syn;
use quote::ToTokens;

fn get_or_make_child_dir<T: AsRef<Path>>(d: T, c: &str) -> Result<PathBuf, Box<dyn Error>> {
    let out = d.as_ref().join(c);
    if !out.exists() { std::fs::create_dir(&out)?; }
    Ok(out)
}

fn generate_codegen_from_ast(mut source_ast: syn::File, to_self: bool) -> String {
    let mut output_file = format!("#![allow(dead_code, improper_ctypes)]
// This file was automatically generated from riri-imgui-hook-globals.\n");
    for item in &mut source_ast.items {
        match item {
            syn::Item::Macro(m) => {
                if m.mac.path.is_ident("create_static") {
                    output_file.push_str(&create_static_links(m.mac.tokens.clone()));
                }
            },
            syn::Item::Use(u) => {
                // check that root of tree is for opengfd crate
                match &mut u.tree {
                    syn::UseTree::Path(p) => if &p.ident.to_string() == "riri-imgui-hook" {
                        if to_self {
                            // we're linking this with OpenGFD itself, so replace opengfd in use with crate
                            p.ident = syn::Ident::new("crate", Span::call_site());
                        }
                        output_file.push_str(&u.to_token_stream().to_string());
                        output_file.push_str("\n");
                    },
                    _ => continue
                }
            },
            _ => continue
        }
    }
    output_file
}

fn save_codegen<P>(path: P, name: &str, output_file: String)
where P: AsRef<Path>
{
    let middata = get_or_make_child_dir(path.as_ref(), "middata").unwrap();
    let output_path = middata.join(name);
    std::fs::write(output_path, output_file).unwrap();
}

fn main() {
    // std::env::var("CARGO_FEATURE_SERVER").unwrap();
    let source_dir = std::env::current_dir().unwrap();
    println!("cargo::rerun-if-changed=src/globals.rs");
    println!("cargo::rerun-if-changed=build.rs");
    let global_source = source_dir.join("src/globals.rs");
    let source_ast = syn::parse_file(&std::fs::read_to_string(global_source).unwrap()).unwrap();
    let glb_self = generate_codegen_from_ast(source_ast.clone(), true);
    let glb_ext = generate_codegen_from_ast(source_ast.clone(), false);
    save_codegen(source_dir.clone(), "self.rs", glb_self);
    save_codegen(source_dir.clone(), "ext.rs", glb_ext);
}

