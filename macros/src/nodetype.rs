use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Block, FnArg, ItemFn, Pat, PatType, ReturnType, Type};
use syn::punctuated::Punctuated;
use syn::token::Comma;

use crate::funcutils::create_dependency_injection_statements;

/// Splits the node function signature inputs into a list of outputs-turn-inputs from the
/// previous node and a set of dependencies to pull from the dependency cache (dependency
/// injection).  Which goes into which is determined by ownership: if the function owns the input,
/// it came from the previous node; if it's a reference, it's coming from the dependency cache.
fn args_to_inputs_and_deps(
    inputs: &Punctuated<FnArg, Comma>
) -> (Punctuated<Box<Type>, Comma>, Vec<PatType>) {
    let mut owned_inputs = Punctuated::<Box<Type>, Comma>::new();
    let mut deps = Vec::<PatType>::new();
    let mut needs_comma = false;
    for arg in inputs.iter() {
        if let FnArg::Typed(pat_type) = &arg {
            match &*pat_type.ty {
                Type::Reference(_) => {
                    deps.push(pat_type.clone());
                },
                _ => {
                    if needs_comma {
                        owned_inputs.push_punct(Comma::default());
                    }
                    owned_inputs.push_value(pat_type.ty.clone());
                    needs_comma = true;
                }
            }
        }
    }
    (owned_inputs, deps)
}

fn inputs_to_names(
    inputs: &Punctuated<FnArg, Comma>
) -> Punctuated<Box<Pat>, Comma> {
    let mut out = Punctuated::<Box<Pat>, Comma>::new();
    let mut needs_comma = false;
    for arg in inputs.iter() {
        if let FnArg::Typed(pat_type) = &arg {
            match &*pat_type.ty {
                Type::Reference(_) => {},
                _ => {
                    if needs_comma {
                        out.push_punct(Comma::default());
                    }
                    out.push_value(pat_type.pat.clone());
                    needs_comma = true;
                }
            }
        }
    }
    out
}

fn output_to_output_type(
    output: &ReturnType
) -> TokenStream {
    match &output {
        ReturnType::Default => quote!{()},
        ReturnType::Type(_, typ) => typ.to_token_stream()
    }
}

fn determine_if_blocking(args: TokenStream) -> bool {
    if args.to_string() == "NONBLOCKING" {false} else {true}
}

fn create_codeblock(is_blocking: bool, deps: Vec<PatType>, code: &Box<Block>) -> TokenStream {
    let code = combine_dep_injection_with_node_code(deps, code);
    if is_blocking {
        quote! {
        {
            tokio::task::spawn_blocking(move ||
            {
                #code
            }
            ).await.unwrap()
        }}
    } else {
        quote!{{#code}}
    }
}

fn code_to_tokenstream(code: &Box<Block>) -> TokenStream {
    let mut out = TokenStream::new();
    for stmt in (*code).stmts.iter() {
        out.extend(quote!{#stmt})
    }
    out
}

fn combine_dep_injection_with_node_code(deps: Vec<PatType>, code: &Box<Block>) -> TokenStream {
    let mut block = create_dependency_injection_statements(deps);
    block.extend(code_to_tokenstream(code));
    block
}

pub fn nodetype_impl(args: TokenStream, func_ast: ItemFn) -> TokenStream {
    let is_blocking = determine_if_blocking(args);
    let vis = &func_ast.vis;
    let name = &func_ast.sig.ident;
    let inputs = &func_ast.sig.inputs;
    let (input_type, deps) = args_to_inputs_and_deps(&inputs);
    let input_names = inputs_to_names(&inputs);
    let output = output_to_output_type(&func_ast.sig.output);
    let code = create_codeblock(is_blocking, deps, &func_ast.block);
    quote! {
        #vis struct #name {}
        #[async_trait::async_trait]
        impl conflagrate::NodeType for #name {
            type Args = (#input_type);
            type ReturnType = #output;
            async fn run((#input_names): Self::Args, _deps: &conflagrate::DependencyCache) -> Self::ReturnType
            #code
        }
    }
}
