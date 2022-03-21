use proc_macro2::{TokenStream as TokenStream2, TokenStream};
use quote::{quote, ToTokens};
use syn::{Block, FnArg, ItemFn, Pat, ReturnType, Type};
use syn::punctuated::Punctuated;
use syn::token::Comma;

fn inputs_to_tuple_type(
    inputs: &Punctuated<FnArg, Comma>
) -> Punctuated<Box<Type>, Comma> {
    let mut out = Punctuated::<Box<Type>, Comma>::new();
    let mut needs_comma = false;
    for typ in inputs.iter() {
        match &typ {
            FnArg::Typed(typ) => {
                if needs_comma {
                    out.push_punct(Comma::default());
                }
                out.push_value(typ.ty.clone());
                needs_comma = true;
            },
            _ => ()
        }
    }
    out
}

fn inputs_to_names(
    inputs: &Punctuated<FnArg, Comma>
) -> Punctuated<Box<Pat>, Comma> {
    let mut out = Punctuated::<Box<Pat>, Comma>::new();
    let mut needs_comma = false;
    for typ in inputs.iter() {
        match &typ {
            FnArg::Typed(typ) => {
                if needs_comma {
                    out.push_punct(Comma::default());
                }
                out.push_value(typ.pat.clone());
                needs_comma = true;
            },
            _ => ()
        }
    }
    out
}

fn output_to_output_type(
    output: &ReturnType
) -> TokenStream2 {
    match &output {
        ReturnType::Default => quote!{()},
        ReturnType::Type(_, typ) => typ.to_token_stream()
    }
}

fn determine_if_blocking(args: TokenStream) -> bool {
    if args.to_string() == "NONBLOCKING" {false} else {true}
}

pub fn create_codeblock(is_blocking: bool, code: &Box<Block>) -> TokenStream {
    if is_blocking {
        quote! {
        {
            tokio::task::spawn_blocking(move ||
                #code
            ).await.unwrap()
        }}
    } else {
        quote!{#code}
    }
}

pub fn nodetype_impl(args: TokenStream, func_ast: ItemFn) -> TokenStream2 {
    let is_blocking = determine_if_blocking(args);
    let name = &func_ast.sig.ident;
    let inputs = &func_ast.sig.inputs;
    let input_type = inputs_to_tuple_type(&inputs);
    let input_names = inputs_to_names(&inputs);
    let output = output_to_output_type(&func_ast.sig.output);
    let code = create_codeblock(is_blocking, &func_ast.block);
    quote! {
        struct #name {}
        #[async_trait::async_trait]
        impl conflagrate::NodeType for #name {
            type Args = (#input_type);
            type ReturnType = #output;
            async fn run((#input_names): Self::Args) -> Self::ReturnType
            #code
        }
    }
}
