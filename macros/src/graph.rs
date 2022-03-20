pub mod graph;
pub mod node;

use crate::graph::graph::Graph;
use proc_macro2::TokenStream;
use quote::quote;

pub fn graph_impl(graph_str: String) -> TokenStream {
    let graph = Graph::from(&graph_str);
    quote! { #graph }
}
