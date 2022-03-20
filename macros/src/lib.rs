mod graph;
mod nodetype;

use crate::nodetype::nodetype_impl;
use crate::graph::graph_impl;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};


#[proc_macro_attribute]
pub fn nodetype(_: TokenStream, func: TokenStream) -> TokenStream {
    TokenStream::from(nodetype_impl(parse_macro_input!(func as ItemFn)))
}

#[proc_macro]
pub fn graph(graph: TokenStream) -> TokenStream {
    TokenStream::from(graph_impl(graph.to_string()))
}
