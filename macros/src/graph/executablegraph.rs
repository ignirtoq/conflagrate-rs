use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use crate::graph::descriptivegraph::DescriptiveGraph;
use crate::graph::task::{Task, TaskName};

/// The execution organized and optimized representation of the control flow graph.
///
/// The `ExecutableGraph` translates directly into compilable Rust code in the form of a public
/// structure with a single `impl` block containing:
/// * A `const SOURCE: &'static str` providing the original Graphviz graph definition text.
/// * A public `run()` method that launches a tokio multi-threaded runtime and runs the graph.
/// * A public `run_graph()` async method that spawns the graph in an already-running tokio
/// runtime and returns the output from the final executed node as its return value.
/// * Private "task" methods each named "execute_{node_name}" that implement the nodes of the
/// control flow graph.
///
/// Note that the task methods may not correspond 1-to-1 with the nodes defined on the graph.
/// The conversion process from the descriptive graph to the executable graph may make some
/// optimizations on the graph structure to generate a more efficient program that's functionally
/// the same.
pub struct ExecutableGraph {
    name: Ident,
    run_method: RunMethod,
    run_graph_method: RunGraphMethod,
    tasks: Vec<Task>,
    source: String,
}
impl From<DescriptiveGraph> for ExecutableGraph {
    fn from(graph: DescriptiveGraph) -> Self {
        let graph_output_type = graph.get_output_type();
        let graph_nodes_map = graph.get_nodes();
        let mut tasks = Vec::<Task>::with_capacity(graph_nodes_map.len());
        for (_, node) in graph_nodes_map {
            tasks.push(Task::from_nodes(&[node.clone()], &graph_output_type));
        }
        Self {
            name: graph.get_name(),
            run_method: RunMethod::from(&graph),
            run_graph_method: RunGraphMethod::from(&graph),
            tasks,
            source: graph.into_source(),
        }
    }
}
impl ToTokens for ExecutableGraph {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let graph_name = &self.name;
        let run_method = &self.run_method;
        let run_graph_method = &self.run_graph_method;
        let tasks = &self.tasks;
        let source = &self.source;
        tokens.extend(quote! {
            pub struct #graph_name;
            impl #graph_name {
                pub const SOURCE: &'static str = #source;
                #run_method
                #run_graph_method
                #(#tasks)*
            }
        })
    }
}

/// Defines the `run()` method of an executable graph.
///
/// Generates a simple wrapper method for the `run_graph()` method that looks like the following:
/// ```no_compile
/// pub fn run(
///     first_node_args: <{start_nodetype} as conflagrate::NodeType>::Args
/// ) {
///     let rt = tokio::runtime::Runtime::new().unwrap();
///     match rt.block_on(async move {
///         Self::run_graph(first_node_args, None).await
///     }) {
///         _ => {}
///     };
/// }
/// ```
struct RunMethod {
    start_nodetype: TokenStream,
}
impl From<&DescriptiveGraph> for RunMethod {
    fn from(graph: &DescriptiveGraph) -> Self {
        Self {
            start_nodetype: graph.get_start_node_nodetype()
        }
    }
}
impl ToTokens for RunMethod {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let start_nodetype = &self.start_nodetype;
        tokens.extend(quote! {
            pub fn run(
                first_node_args: <#start_nodetype as conflagrate::NodeType>::Args
            ) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(async move {
                    Self::run_graph(first_node_args, None).await
                }) {
                    _ => {}
                };
            }
        });
    }
}

/// Defines the `run_graph()` async method on an executable graph.
///
/// Generates a method definition that looks roughly like the following:
/// ```no_compile
/// pub async fn run_graph(
///     first_node_args: <{start_nodetype} as conflagrate::NodeType>::Args,
///     dependency_cache: Option<std::sync::Arc<conflagrate::DependencyCache>>
/// ) -> Result<{graph_output_type}, tokio::sync::oneshot::error::RecvError> {
///     let (receiver, raw_branch_tracker) = conflagrate::BranchTracker::<{graph_output_type}>::new();
///     let branch_tracker = std::sync::Arc::new(tokio::sync::Mutex::new(raw_branch_tracker));
///     let deps = match dependency_cache {
///         Some(deps) => deps,
///         None => std::sync::Arc::new(conflagrate::DependencyCache::new()),
///     };
///     tokio::spawn(async move {
///         Self::execute_{start_node_name}(branch_tracker, first_node_args, deps).await;
///     });
///     receiver.await
/// }
/// ```
struct RunGraphMethod {
    start_nodetype: TokenStream,
    graph_output_type: TokenStream,
    start_node_name: TaskName,
}
impl From<&DescriptiveGraph> for RunGraphMethod {
    fn from(graph: &DescriptiveGraph) -> Self {
        Self {
            start_nodetype: graph.get_start_node_nodetype(),
            graph_output_type: graph.get_output_type(),
            start_node_name: TaskName::from(graph.get_start_node_name())
        }
    }
}
impl ToTokens for RunGraphMethod {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let start_nodetype = &self.start_nodetype;
        let graph_output_type = &self.graph_output_type;
        let execute_start_node = &self.start_node_name;
        tokens.extend(quote! {
            pub async fn run_graph(
                first_node_args: <#start_nodetype as conflagrate::NodeType>::Args,
                dependency_cache: Option<std::sync::Arc<conflagrate::DependencyCache>>
            ) -> Result<#graph_output_type, tokio::sync::oneshot::error::RecvError> {
                let (receiver, raw_branch_tracker) = conflagrate::BranchTracker::<#graph_output_type>::new();
                let branch_tracker = std::sync::Arc::new(tokio::sync::Mutex::new(raw_branch_tracker));
                let deps = match dependency_cache {
                    Some(deps) => deps,
                    None => std::sync::Arc::new(conflagrate::DependencyCache::new()),
                };
                tokio::spawn(async move {
                    Self::#execute_start_node(branch_tracker, first_node_args, deps).await;
                });
                receiver.await
            }
        })
    }
}
