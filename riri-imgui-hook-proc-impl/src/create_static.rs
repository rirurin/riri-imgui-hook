use proc_macro2::{Span, TokenStream};
use syn::{
    parse::{ Parse, Parser }, Token
};
use quote::{quote, ToTokens};
pub struct GfdStatic {
    name: String,
    name_lower: String,
    data: GfdStaticData
}

impl GfdStatic {

    fn get_checked_some_pointer(&self, is_mutable: bool) -> TokenStream {
        match &self.data {
            GfdStaticData::Constant(_) => {
                let deref = if is_mutable { 
                    quote! { &mut *v.0 } 
                } else { 
                    quote!{ &*v.0 } 
                };
                quote! { Some(v) => Some(unsafe { #deref }) }
            },
            GfdStaticData::Singleton(_) => {
                let deref = if is_mutable { 
                    quote! { &mut **v.0 } 
                } else { 
                    quote!{ &**v.0 } 
                };
                quote! { Some(v) => { if !v.0.is_null() {Some(unsafe{#deref})} else {None} } }
            }
        }
    }

    fn get_checked_pointer_function_name(&self, is_mutable: bool) -> String {
        if is_mutable { format!("get_{}_mut", self.name_lower) }
        else { format!("get_{}", self.name_lower) }
    }

    fn generate_get_checked_pointer_function(&self, is_mutable: bool, comment: String) -> syn::Result<TokenStream> {
        let fn_name = syn::Ident::new(&self.get_checked_pointer_function_name(is_mutable),
            Span::call_site());
        let glb_name = syn::Ident::new(&self.name, Span::call_site());
        let out_type = self.data.get_return_type_as_tokens();
        let ref_type = if is_mutable {
            quote! { &'static mut }
        } else {
            quote! { &'static }
        };
        let some_res = self.get_checked_some_pointer(is_mutable);
        let comment: TokenStream = comment.parse().unwrap();
        Ok(quote! {
            #comment
            #[no_mangle]
            pub unsafe extern "C" fn #fn_name() -> Option<#ref_type #out_type> {
                match #glb_name.get() {
                    #some_res,
                    None => None
                }
            }
        })
    }

    fn get_unchecked_pointer_function_name(&self, is_mutable: bool) -> String {
        if is_mutable { format!("get_{}_unchecked_mut", self.name_lower) }
        else { format!("get_{}_unchecked", self.name_lower) }
    }

    fn generate_get_unchecked_pointer_function(&self, is_mutable: bool, comment: String) -> syn::Result<TokenStream> {
        let fn_name = syn::Ident::new(&self.get_unchecked_pointer_function_name(is_mutable),
            Span::call_site());
        let glb_name = syn::Ident::new(&self.name, Span::call_site());
        let out_type = self.data.get_return_type_as_tokens();
        let ref_type = if is_mutable {
            quote! { &'static mut }
        } else {
            quote! { &'static }
        };
        let deref = match &self.data {
            GfdStaticData::Constant(_) => {
                if is_mutable { quote! { &mut * } } else { quote!{ &* } }
            },
            GfdStaticData::Singleton(_) => {
                if is_mutable { quote! { &mut ** } } else { quote!{ &** } }
            }
        };
        let comment: TokenStream = comment.parse().unwrap();
        Ok(quote! {
            #comment
            #[no_mangle]
            pub unsafe extern "C" fn #fn_name() -> #ref_type #out_type {
                #deref #glb_name.get().unwrap().0
            }
        })
    }

    fn get_set_pointer_name(&self) -> String { format!("set_{}", self.name_lower) }

    fn get_set_name_comment(&self) -> String {
        match &self.data {
            GfdStaticData::Constant(_) => format!("/// Set the pointer to the memory location containing the beginning of {}.
    /// This method must only be called once, otherwise it will panic.", &self.name),
            GfdStaticData::Singleton(_) => format!("/// Set the pointer to the memory location containing a pointer to {}.
    /// This method must only be called once, otherwise it will panic.", &self.name)
        }
    }

    fn create_set_name_tokens(&self) -> syn::Result<TokenStream> {
        let fn_name = syn::Ident::new(&self.get_set_pointer_name(), Span::call_site());
        let comment = self.get_set_name_comment();
        let glb_name = syn::Ident::new(&self.name, Span::call_site());
        let type_param = self.data.get_full_type_as_tokens();
        let comment: TokenStream = comment.parse().unwrap();
        Ok(quote! {
            #comment
            #[no_mangle]
            pub(crate) unsafe extern "C" fn #fn_name(ptr: *mut #type_param) {
                #glb_name.set(crate::UnsafePtr(ptr)).unwrap();
            }
        })
    }

    fn create_get_comment(&self) -> String {
        format!("/// Get a possible reference to {}. This checks to see if `{}`
    /// was called previously and if either you or the hooked process have allocated the instance of it.",
                &self.name, &self.get_set_pointer_name())
    }

    fn create_get_tokens(&self) -> syn::Result<TokenStream> {
        self.generate_get_checked_pointer_function(false, self.create_get_comment())
    }

    fn create_get_mut_comment(&self) -> String {
        format!("/// Like `{}`, but a mutable reference is created instead.",
                &self.get_checked_pointer_function_name(true)
        )
    }

    fn create_get_mut_tokens(&self) -> syn::Result<TokenStream> {
        self.generate_get_checked_pointer_function(true, self.create_get_mut_comment())
    }

    fn create_get_unchecked_comment(&self) -> String {
        format!("/// An unchecked version of `{}`. This assumes that {}
    /// is set and it's initialized.",
                &self.get_checked_pointer_function_name(false), &self.name
        )
    }

    fn create_get_unchecked_tokens(&self) -> syn::Result<TokenStream> {
        self.generate_get_unchecked_pointer_function(false, self.create_get_unchecked_comment())
    }

    fn create_get_unchecked_mut_comment(&self) -> String {
        format!("/// An unchecked version of `{}`. This assumes that {}
    /// is set and it's initialized.",
                &self.get_checked_pointer_function_name(true), &self.name
        )
    }

    fn create_get_mut_unchecked_tokens(&self) -> syn::Result<TokenStream> {
        self.generate_get_unchecked_pointer_function(true, self.create_get_unchecked_mut_comment())
    }   

    pub fn codegen(&self) -> syn::Result<TokenStream> {
        // Create declaration (OnceLock)
        let decl_name = syn::Ident::new(&self.name, Span::call_site());
        let decl_type = self.data.get_full_type_as_tokens();
        let decl = quote! {
            #[doc(hidden)]
            static #decl_name: ::std::sync::OnceLock<crate::UnsafePtr<#decl_type>> = ::std::sync::OnceLock::new();
        };
        // Make get/set functions
        let set_fn = self.create_set_name_tokens()?;
        let get_fn = self.create_get_tokens()?;
        let get_mut_fn = self.create_get_mut_tokens()?;
        let get_unchecked_fn = self.create_get_unchecked_tokens()?;
        let get_mut_unchecked_fn = self.create_get_mut_unchecked_tokens()?;
        let out = quote! {
            #decl
            #set_fn
            #get_fn
            #get_mut_fn
            #get_unchecked_fn
            #get_mut_unchecked_fn
        };
        Ok(out)
    }

    fn create_set_name_tokens_link(&self) -> String {
        let type_param = self.data.get_full_type_as_tokens();
        format!("   {}\n    pub(crate) unsafe fn {}(ptr: *mut {});\n", self.get_set_name_comment(), self.get_set_pointer_name(), type_param)
    }

    fn generate_get_pointer_link(&self, is_mutable: bool, is_checked: bool, name: String, comment: String) -> String {
        let out_type = self.data.get_return_type_as_tokens();
        let ref_type = if is_mutable { quote! { &'static mut } } else { quote! { &'static } };
        if is_checked {
            format!("   {}\n    pub(crate) unsafe fn {}() -> Option<{} {}>;\n", comment, name, ref_type, out_type)
        } else {
            format!("   {}\n    pub(crate) unsafe fn {}() -> {} {};\n", comment, name, ref_type, out_type)
        }
        
    }

    pub fn link_codegen(&self) -> syn::Result<String> {
        let mut link_data = format!("#[link(name = \"cri_adx_globals\", kind = \"raw-dylib\")]\n");
        link_data.push_str("unsafe extern \"C\" {\n");
        link_data.push_str(&self.create_set_name_tokens_link());
        link_data.push_str(&self.generate_get_pointer_link(false, true,
            self.get_checked_pointer_function_name(false), self.create_get_comment()));
        link_data.push_str(&self.generate_get_pointer_link(true, true,
            self.get_checked_pointer_function_name(true), self.create_get_mut_comment()));
        link_data.push_str(&self.generate_get_pointer_link(false, false,
            self.get_unchecked_pointer_function_name(false), self.create_get_unchecked_comment()));
        link_data.push_str(&self.generate_get_pointer_link(true, false,
            self.get_unchecked_pointer_function_name(true), self.create_get_unchecked_mut_comment()));
        link_data.push_str("\n}\n\n");
        Ok(link_data)
    }
}

pub enum GfdStaticData {
    Constant(syn::TypePath),
    Singleton(syn::TypePtr)
}


impl GfdStaticData {  
    fn get_full_type_as_tokens(&self) -> TokenStream {
        match self {
            GfdStaticData::Constant(c) => c.to_token_stream(),
            GfdStaticData::Singleton(s) => s.to_token_stream()
        }
    }
    fn get_return_type_as_tokens(&self) -> TokenStream {
        match self {
            GfdStaticData::Constant(c) => c.to_token_stream(),
            GfdStaticData::Singleton(s) => s.elem.to_token_stream()
        }
    }
}


impl Parse for GfdStatic {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if !input.peek(syn::Ident) {
            return Err(syn::Error::new(input.span(), "First parameter must be a valid variable name"))
        } 
        let name = input.parse::<syn::Ident>()?.to_string();
        input.parse::<Token![,]>()?;
        let ty = input.parse()?;
        let data = match ty {
            syn::Type::Path(p) => GfdStaticData::Constant(p),
            syn::Type::Ptr(p) => {
                if p.const_token.is_some() {
                    return Err(syn::Error::new(input.span(), "Pointer type must be mutable"))
                }
                if let syn::Type::Path(_) = p.elem.as_ref() {
                    GfdStaticData::Singleton(p)
                } else {
                    return Err(syn::Error::new(input.span(), "Pointer should only have one level of indirection"))
                }
            },
            _ => return Err(syn::Error::new(input.span(), "Type must be either a path or pointer"))
        };
        let name_lower = name.to_ascii_lowercase();
        Ok(Self { name, name_lower, data })
    }
}

pub fn create_static(input: TokenStream) -> TokenStream {
    let info = match GfdStatic::parse.parse2(input) {
       Ok(s) => s,
       Err(e) => return e.to_compile_error()
    };
    match info.codegen() {
        Ok(v) => v,
        Err(e) => return e.to_compile_error()
    }
}

pub fn create_static_links(input: TokenStream) -> String {
    let info = match GfdStatic::parse.parse2(input) {
        Ok(s) => s,
        Err(_) => panic!("Error while parsing macro input for create_static_links")
    };
    info.link_codegen().unwrap()
}

