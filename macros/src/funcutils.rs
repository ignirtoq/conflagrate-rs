use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{PatType, Type};

fn dep_to_dependency_injection_statements(dep: &PatType) -> TokenStream {
    let name = &dep.pat;
    let name_quoted = name.to_token_stream().to_string();
    let typ = match &*dep.ty {
        Type::Reference(type_reference) => &type_reference.elem,
        _ => panic!("tried to make a dependency out of a type that's not a reference!")
    };
    quote! {
        if !_deps.contains(#name_quoted).await { #name(_deps).await }
        let #name = _deps.get::<#typ>(#name_quoted).await.unwrap();
    }
}

pub fn create_dependency_injection_statements(deps: Vec<PatType>) -> TokenStream {
    let mut out = TokenStream::new();
    for dep in deps.iter() {
        out.extend(dep_to_dependency_injection_statements(&dep));
    }
    out
}
