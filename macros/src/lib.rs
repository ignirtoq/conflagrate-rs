//! See the [conflagrate](https://docs.rs/conflagrate) documentation for details about the macros
//! contained in this crate.
mod dependency;
mod funcutils;
mod graph;
mod nodetype;

use crate::dependency::dependency_impl;
use crate::graph::graph_impl;
use crate::nodetype::nodetype_impl;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};


#[doc(hidden)]
#[proc_macro_attribute]
pub fn dependency(_: TokenStream, func: TokenStream) -> TokenStream {
    TokenStream::from(dependency_impl(parse_macro_input!(func as ItemFn)))
}

#[doc(hidden)]
#[proc_macro_attribute]
pub fn nodetype(_: TokenStream, func: TokenStream) -> TokenStream {
    TokenStream::from(
        nodetype_impl(parse_macro_input!(func as ItemFn))
    )
}

#[doc(hidden)]
#[proc_macro]
pub fn graph(graph: TokenStream) -> TokenStream {
    TokenStream::from(graph_impl(graph.to_string()))
}
