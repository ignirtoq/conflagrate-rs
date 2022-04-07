mod dependency;
mod funcutils;
mod graph;
mod nodetype;

use crate::dependency::dependency_impl;
use crate::graph::graph_impl;
use crate::nodetype::nodetype_impl;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{parse_macro_input, ItemFn};


#[proc_macro_attribute]
pub fn dependency(_: TokenStream, func: TokenStream) -> TokenStream {
    TokenStream::from(dependency_impl(parse_macro_input!(func as ItemFn)))
}

#[proc_macro_attribute]
pub fn nodetype(args: TokenStream, func: TokenStream) -> TokenStream {
    TokenStream::from(
        nodetype_impl(TokenStream2::from(args), parse_macro_input!(func as ItemFn))
    )
}

#[proc_macro]
pub fn graph(graph: TokenStream) -> TokenStream {
    TokenStream::from(graph_impl(graph.to_string()))
}
