//! Conflagrate is a framework for building applications from control flow graphs, instead of
//! the other way around.
//!
//! Conflagrate is intended to bring modularity, maintainability, and extensibility to control flow
//! logic.  Where object oriented design, functional programming, dependency injection, and similar
//! tools help developers build and reuse functionality, conflagrate helps developers to encapsulate
//! that functionality in modular, testable building blocks that can be arranged and rearranged to
//! construct a complete application.  Importantly, this application can be easily updated to change
//! the order of operations, conditions of branch execution, or even to add entirely new subsystems
//! without needing to refactor the existing components or control flow.
//!
//! The control flow graph is defined with the [`graph`] macro, which converts your graph as
//! defined with the [DOT language](https://graphviz.org) into an executable structure.  Each
//! node in graph is annotated with a [`nodetype`] that associates a block of code with the node.
//! Finally, sometimes a node needs more to do its job than just the output of the previous node
//! in the graph.  Conflagrate provides a simple dependency injection system to provide external
//! resources and data.  Define a dependency provider function with the [`dependency`] macro and
//! add a reference to the dependency to the [`nodetype`] function signature.

mod branchtracker;
mod dependencies;

/// Defines a dependency that can be accessed from any node in the graph.
///
/// Dependencies are resources shared amongst all nodes in a graph.  They can be interfaces to
/// external processes or collections of data shared between disparate parts of the graph.
/// Basically anything a node needs that isn't provided directly by the preceding node should be
/// provided as a dependency.
///
/// # Providers
///
/// Async functions decorated with `#[dependency]` macro construct a dependency that shares the
/// name of the function.  The first time a node that declares that dependency is executed in a
/// graph, the graph will call the provider function and store the returned object in the
/// dependency cache.  The cache is a hash map whose keys are the names of the provider functions
/// and the values are the provided objects.
///
/// # Shared Resources
///
/// Dependencies are only created the first time they are needed.  Every node that names the same
/// dependency gets the same object.  Because multiple nodes can be running simultaneously on
/// separate threads, nodes only receive the dependency as an immutable reference.  If you need
/// mutable access to the dependency, wrap it in a
/// [`tokio::sync::Mutex`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html).
///
/// # Examples
/// ```
/// use conflagrate::{dependency, graph, nodetype};
/// use tokio::sync::Mutex;
///
/// #[dependency]
/// async fn messages() -> Mutex<Vec<String>> {
///     Mutex::<Vec<String>>::new(Vec::<String>::new())
/// }
///
/// #[nodetype]
/// pub async fn StoreMessage(messages: &Mutex<Vec<String>>) {
///     messages.lock().await.push(String::from("Hello mutable storage!"));
/// }
///
/// #[nodetype]
/// async fn StoreAnotherMessage(messages: &Mutex<Vec<String>>) {
///     messages.lock().await.push(String::from("Here's another message!"));
/// }
///
/// #[nodetype]
/// pub async fn PrintMessages(messages: &Mutex<Vec<String>>) {
///     for msg in messages.lock().await.iter() {
///         println!("{}", msg);
///     }
/// }
///
/// graph!{
///     digraph MessageGraph {
///         store[type=StoreMessage, start=true];
///         store_another[type=StoreAnotherMessage];
///         print[type=PrintMessages];
///
///         store -> store_another -> print;
///     }
/// }
///
/// fn main() {
///     MessageGraph::run(());
/// }
/// ```
///
/// # See Also
/// * [`nodetype`] -- Macro for associating a function with a type of node.  Reference types in
/// input arguments are interpreted to be dependencies.
pub use conflagrate_macros::dependency;

/// Defines the control flow graph of an application.
///
/// The `graph!` macro defines a control flow graph that can be run as a stand-alone application
/// or called as a subgraph from within another graph.  The syntax follows the standard
/// [DOT language](https://graphviz.org) excepting a few new, non-standard attributes that can
/// be applied to nodes and edges that conflagrate uses to construct the executable application
/// logic.
///
/// # Node Attributes
///
/// * `type` -- The [`nodetype`] associated with the node the in graph, which is a block of
/// executable code that takes as input the output from the previous node and provides as output
/// the input to the next node.
/// * `start` -- Labels the node to start the graph from.  Only one node may be labeled with the
/// `start` attribute.
/// * `branch` -- Tells conflagrate how to handle a node that has more than one node trailing it in
/// the graph.  May take the following values:
///     * `parallel` (default) -- Conflagrate executes all trailing nodes simultaneously in
/// parallel.  The return value from the node is cloned and passed separately to each tail.  If
/// the `branch` attributed is omitted, this value is assumed.
///     * `matcher` -- Conflagrate executes only one trailing node.  The choice of node depends on
/// the value returned by the `nodetype` function.  Edges tagged with the `value` attribute (see
/// below) are matched against the returned value, and a matching edge determines the following
/// node.  If no matches are found, the edge without a `value` attribute is used as the default.
///
/// # Edge Attributes
///
/// * `value` -- Used with nodes with the `branch=matcher` attribute (see above).  The return value
/// of the matcher node is compared against this (string) value.  If it matches, this edge is
/// followed to determine the next node to be executed in the graph.
///
/// # Examples
///
/// ## Trivial Graph
///
/// A single-node graph that just prints the text `Hello, world!`.
/// ```
/// # use conflagrate::{graph, nodetype};
/// #[nodetype]
/// pub fn HelloWorld() {
///     println!("Hello, world!");
/// }
///
/// graph!{
///     digraph {
///         start[type=HelloWorld, start=true];
///     }
/// }
///
/// fn main() {
///     Graph::run(());
/// }
/// ```
///
/// ## Simple Loop
///
/// A simple loop that asks the user if they wish to exit (type `yes` to exit).
/// ```no_run
/// # use conflagrate::{graph, nodetype};
/// #[nodetype]
/// pub fn AskExit() -> (String, ()) {
///     let mut response = String::new();
///     println!("Exit loop?");
///     std::io::stdin().read_line(&mut response).unwrap();
///     response.truncate(response.len() - 1);
///     (response, ())
/// }
///
/// #[nodetype]
/// pub async fn DoNothing() {}
///
/// graph!{
///     digraph Loop {
///         ask_exit[type=AskExit, branch=matcher, start=true];
///         end[type=DoNothing];
///
///         ask_exit -> end [value=yes];
///         ask_exit -> ask_exit;
///     }
/// }
///
/// fn main() {
///     Loop::run(())
/// }
/// ```
///
/// This graph makes use of the `matcher` branching logic.  The `ask_exit` node has two tails,
/// one that goes to the `end` node with the `value=yes` attribute, and another that goes back to
/// the `ask_exit` node with no attributes.  The matching logic chooses which trailing node to
/// execute next based on the first element in the tuple output from the `AskExit` nodetype
/// function.  The second element of the tuple is then passed as input to the chosen following node.
///
/// # See Also
///
/// * [`nodetype`] -- Macro associating functions with nodes.
/// * [`dependency`] -- Macro associating a function providing a resource with the name of the
/// function, providing that resource to `nodetype`s that reference them.
pub use conflagrate_macros::graph;

/// Defines a block of code to be associated with nodes of a certain type in a graph.
///
/// Each executable node in a graph is given a `type`, which is a non-standard GraphViz attribute
/// that conflagrate associates with a function of the same name passed to the [`nodetype`] macro.
///
/// The input of a node is the output of the previous node in the graph (or the graph's input),
/// and the output of the node is the input of the next node in the graph (or the return value of
/// the graph).
///
/// # Blocking Versus Non-Blocking
///
/// Conflagrate applications are built using `tokio`, so `nodetype`s are converted to async
/// functions.  If a regular function is passed into the `nodetype` macro, conflagrate assumes
/// it is blocking and spawns its codeblock in a separate thread.  To avoid spawning extra
/// threads, use `async fn` wherever possible.
///
/// # Visibility (Public Versus Private)
///
/// To facilitate larger projects split into multiple modules, the `run` and `run_graph` methods
/// of [`graph`]s are `pub`.  The arguments to the graph are the input arguments to the first node,
/// and the return value of the graph is the return value of the last possible node, so
/// consequently the `nodetype` functions of those nodes must also be `pub`.
///
/// # Testing
///
/// Under the hood, the `nodetype` function is converted to a struct implementing a trait that
/// provides the function with a uniform call signature so that when it's used with the [`graph`]
/// macro, the graph builder doesn't need to know anything about the shape of your function.  This
/// makes testing more difficult, so your original function is also provided as a `test` method.
/// The `test` method has exactly the same call signature as the original definition.
///
/// ```
/// # use conflagrate::{nodetype};
/// #[nodetype]
/// async fn BusinessLogic(value: u32) -> Result<String, String> {
///     match value {
///         0..=10 => Ok(String::from("good")),
///         _ => Err(String::from("too high!"))
///     }
/// }
///
/// #[cfg(test)]
/// mod tests {
///     # use std::assert_eq;
///     # use super::BusinessLogic;
///     #[test]
///     fn handles_good_values() {
///         assert_eq!(BusinessLogic::test(1), Ok(String::from("good")));
///         assert_eq!(BusinessLogic::test(10), Ok(String::from("good")));
///     }
///
///     #[test]
///     fn bails_on_bad_values() {
///         assert_eq!(BusinessLogic::test(100), Err(String::from("too high!")));
///     }
/// }
/// ```
///
/// # Examples
/// ```no_run
/// # use conflagrate::{graph, nodetype};
/// #[nodetype]
/// pub fn GetName() -> String {
///    let mut name = String::new();
///    println!("Hello, what is your name?");
///    std::io::stdin().read_line(&mut name).unwrap();
///    name.truncate(name.len() - 1);
///    name
/// }
///
/// #[nodetype]
/// pub fn PrintGreeting(name: String) {
///     println!("Hello, {}!", name)
/// }
///
/// graph!{
///     digraph GreetingGraph {
///         get_name[type=GetName, start=true];
///         print_name[type=PrintGreeting];
///
///         get_name -> print_name;
///     }
/// }
///
/// fn main() {
///     GreetingGraph::run(());
/// }
/// ```
///
/// # See Also
/// * [`dependency`] -- Macro for defining an external resource or data to be used as input by a
/// node in addition to the output of the previous node.
/// * [`graph`] -- Macro for building an executable control flow graph using `nodetype`s.
pub use conflagrate_macros::nodetype;
#[doc(hidden)]
pub use branchtracker::BranchTracker;
#[doc(hidden)]
pub use dependencies::DependencyCache;

#[doc(hidden)]
#[async_trait::async_trait]
pub trait NodeType {
    type Args: Clone;
    type ReturnType;
    async fn run(args: Self::Args, deps: &DependencyCache) -> Self::ReturnType;
}
