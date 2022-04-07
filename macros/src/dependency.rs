use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, PatType, Type};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use crate::funcutils::create_dependency_injection_statements;

fn args_to_deps(
    inputs: &Punctuated<FnArg, Comma>
) -> Vec<PatType> {
    let mut deps = Vec::<PatType>::new();
    for arg in inputs.iter() {
        if let FnArg::Typed(pat_type) = &arg {
            if let Type::Reference(_) = &*pat_type.ty {
                deps.push(pat_type.clone());
            }
        }
    }
    deps
}

pub fn dependency_impl(func_ast: ItemFn) -> TokenStream {
    let vis = &func_ast.vis;
    let name = &func_ast.sig.ident;
    let name_quoted = name.to_string();
    let deps = args_to_deps(&func_ast.sig.inputs);
    let dep_injection_stmts = create_dependency_injection_statements(deps);
    let code = &func_ast.block;
    quote!{
        #vis async fn #name(_deps: &conflagrate::DependencyCache) {
            #dep_injection_stmts
            let #name = #code;
            _deps.insert(#name_quoted, #name).await
        }
    }
}
