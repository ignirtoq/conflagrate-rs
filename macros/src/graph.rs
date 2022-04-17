mod descriptivegraph;
mod executablegraph;
mod node;
mod task;

use crate::graph::descriptivegraph::DescriptiveGraph;
use proc_macro2::TokenStream;
use quote::quote;
use crate::graph::executablegraph::ExecutableGraph;

pub fn graph_impl(graph_str: String) -> TokenStream {
    let graph = ExecutableGraph::from(DescriptiveGraph::from(&graph_str));
    quote! { #graph }
}
