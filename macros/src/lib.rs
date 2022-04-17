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
/// * [`nodetype`](macro@nodetype) -- Macro for associating a function with a type of node.
/// Reference types in
/// input arguments are interpreted to be dependencies.
#[proc_macro_attribute]
pub fn dependency(_: TokenStream, func: TokenStream) -> TokenStream {
    TokenStream::from(dependency_impl(parse_macro_input!(func as ItemFn)))
}

/// Defines a block of code to be associated with nodes of a certain type in a graph.
///
/// Each executable node in a graph is given a `type`, which is a non-standard GraphViz attribute
/// that conflagrate associates with a function of the same name passed to the
/// [`nodetype`](macro@nodetype) macro.
///
/// The input of a node is the output of the previous node in the graph (or the graph's input),
/// and the output of the node is the input of the next node in the graph (or the return value of
/// the graph).
///
/// ```no_run
/// # use std::collections::VecDeque;
/// # use conflagrate::nodetype;
/// #[nodetype]
/// async fn SplitCommand(input: String) -> VecDeque<String> {
///     input.split_whitespace().map(String::from).collect::<VecDeque<String>>()
/// }
/// ```
///
/// # Branching Behavior
///
/// In a graph, when one node has more than one arrow following it pointing to different nodes,
/// then the graph has "branched".  Branches in control flow graphs can have multiple,
/// contradictory meanings.  In some cases, we may use branches to show conditional execution: if
/// some condition is satisfied, follow one branch, otherwise follow another.  In other cases
/// branching may represent parallel execution: at this point in the application, spawn N tasks
/// and copy the data to each task.
///
/// To cover these possibilities, conflagrate supports tagging the nodes in a graph with a
/// `branch` attribute to specify what the node's branching behavior should be (see [`graph`:
/// Node Attributes](graph#node-attributes)).  Some choices of branching behavior require a
/// `nodetype`'s output to be structured a certain way.
///
/// ## Parallel
///
/// The default branching behavior is simply to spawn a separate task for each following node in
/// parallel.  The output from the branching node is cloned to each trailing node.
///
/// Parallel branching puts no constraints on the return type of the node, other than the usual
/// requirement that each following node must accept exactly the same number and types as their
/// input.
///
/// ## Matcher
///
/// If a node is given the `branch=matcher` attribute in the graph definition, it is specified to
/// have the `matcher` branching behavior, where conflagrate executes only one trailing node
/// determined by the output of the matcher node.  This puts a constraint on the form of the
/// output of the node.
///
/// The return type of the node is required to be a 2-tuple of the form `(String, T)`, where the
/// String first element is used for matching and the `T` second element is passed to the next node
/// as its input.  Edges tagged with the `value` attribute (see
/// [`graph`: Edge Attributes](graph#edge-attributes)) are matched using the attribute's value,
/// and a matching edge determines the following node.  If no matches are found, an edge without a
/// `value` attribute is used as the default. If no default edge is provided, the graph will
/// terminate, using the matcher node's output as its output.
///
/// ```
/// # use std::collections::VecDeque;
/// # use conflagrate::nodetype;
/// #[nodetype]
/// async fn GetCommand(input: VecDeque<String>) -> (String, VecDeque<String>) {
///     let cmd = input.pop_front().unwrap_or(String::from(""));
///     (cmd, input)
/// }
/// ```
///
/// ## Result Matcher
///
/// Like the matcher behavior, if a node is described with the `branch=resultmatcher` attribute,
/// the choice of trailing nodes depends on the output of the node.  In this case, the return
/// type must be a single [`Result<T, E>`](core::result).  Edges marked with the `value=ok`
/// attribute will match against the `Ok(T)` variant and receive `T` as their input type, and edges
/// with `value=err` will match `Err(E)` and receive `E` as their input type.  Unlike
/// the regular matcher type, multiple trailing nodes can be labeled with either `value=ok` or
/// `value=err` on their edges, allowing for parallel execution as in the default parallel
/// branching behavior.
///
/// ```
/// # use std::collections::VecDeque;
/// # use conflagrate::nodetype;
/// #[nodetype]
/// async fn GetCommand(input: VecDeque<String>) -> Result<(String, VecDeque<String>), String> {
///     match input.pop_front() {
///         Ok(cmd) => Ok((cmd, input)),
///         Err(e) => Err(format!("unable to get command from input: {}", e.to_string())),
///     }
/// }
/// ```
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
/// of [`graph`](macro@graph)s are `pub`.  The arguments to the graph are the input arguments to
/// the first node, and the return value of the graph is the return value of the last possible node,
/// so consequently the `nodetype` functions of those nodes must also be `pub`.
///
/// # Testing
///
/// Under the hood, the `nodetype` function is converted to a struct implementing a trait that
/// provides the function with a uniform call signature so that when it's used with the
/// [`graph`](macro@graph) macro, the graph builder doesn't need to know anything about the shape of
/// your function.  This makes testing more difficult, so your original function is also provided as
/// a `test` static method. The `test` method has exactly the same call signature as the original
/// definition.
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
///
/// ## Hello World
///
/// A simple "hello world" graph with two nodes, `get_name` and `print_name`.  These nodes have
/// the `nodetype`s `GetName` and `PrintGreeting`, respectively.  The graph starts at `get_name`
/// and then follows to `print_name`.  The `GetName` `nodetype` returns a `String`, so the
/// `nodetype` of the node that follows, `PrintGreeting` for `print_name`, must take as its input
/// just a `String` (plus an dependencies, but in this case there are none).
///
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
/// * [`dependency`](macro@dependency) -- Macro for defining an external resource or data to be used
/// as input by a node in addition to the output of the previous node.
/// * [`graph`](macro@graph) -- Macro for building an executable control flow graph using
/// `nodetype`s.
#[proc_macro_attribute]
pub fn nodetype(_: TokenStream, func: TokenStream) -> TokenStream {
    TokenStream::from(
        nodetype_impl(parse_macro_input!(func as ItemFn))
    )
}

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
/// * `type` -- The [`nodetype`](macro@nodetype) associated with the node the in graph, which is a
/// block of executable code that takes as input the output from the previous node and provides as
/// output the input to the next node.  Multiple nodes in the graph can use the same `nodetype`
/// to facillitate more code reuse.
/// * `start` -- Labels the node to start the graph from.  Only one node may be labeled with the
/// `start` attribute.
/// * `branch` -- Tells conflagrate how to handle a node that has more than one node trailing it in
/// the graph.  May take the following values:
///     * `parallel` (default) -- Conflagrate executes all trailing nodes simultaneously in
/// parallel.  The return value from the node is cloned and passed separately to each tail.  If
/// the `branch` attribute is omitted, this value is assumed.
///     * `matcher` -- Conflagrate executes only one trailing node determined by the output of
/// the matcher node.  This puts constraints on the required return type of the `nodetype` (see
/// [`nodetype`: Matcher](nodetype#matcher)).
///     * `resultmatcher` -- A variant of `matcher` that matches on a `Result` instead of a
/// `String` (see [`nodetype`: Result Matcher](nodetype#result-matcher)).
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
/// * [`nodetype`](macro@nodetype) -- Macro associating functions with nodes.
/// * [`dependency`](macro@nodetype) -- Macro associating a function providing a resource with the
/// name of the function, providing that resource to `nodetype`s that reference them.
#[proc_macro]
pub fn graph(graph: TokenStream) -> TokenStream {
    TokenStream::from(graph_impl(graph.to_string()))
}
