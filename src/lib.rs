//! Conflagrate is a framework for building applications from control flow graphs, instead of
//! the other way around.
//!
//! Conflagrate is designed to bring modularity, maintainability, and extensibility to control flow
//! logic.  Where object oriented design, functional programming, dependency injection, and similar
//! tools help developers build and reuse functionality, conflagrate helps developers to encapsulate
//! that functionality in modular, testable building blocks that can be arranged and rearranged to
//! construct a complete application.  Importantly, this application's control flow can be easily
//! updated to change the order of operations, conditions of branch execution, or even to add
//! entirely new subsystems without needing to refactor existing components or control flow.
//!
//! The control flow graph is defined with the [`graph`] macro, which converts your graph as
//! defined with the [DOT language](https://graphviz.org) into an executable structure.  Each
//! node in the graph is annotated with a [`nodetype`] that associates a block of code with the
//! node. Finally, sometimes a node needs more to do its job than just the output of the previous
//! node in the graph.  Conflagrate provides a simple dependency injection system to provide
//! external resources and data.  Define a dependency provider function with the [`dependency`]
//! macro and add a reference to the dependency to the [`nodetype`] function signature.
//!
//! # Examples
//!
//! A simple program to record console inputs on a loop and echo them all back on exit:
//! ```no_run
//! # use tokio::sync::Mutex;
//! # use conflagrate::{dependency, graph, nodetype};
//! #
//! #[dependency]
//! async fn memory() -> Mutex<Vec<String>> {
//!     Mutex::new(Vec::<String>::new())
//! }
//!
//! #[nodetype]
//! pub fn GetInput() -> String {
//!     let mut input = String::new();
//!     println!("Type any input ('exit' to exit):");
//!     std::io::stdin().read_line(&mut input).unwrap();
//!     input.truncate(input.len() - 1);
//!     input
//! }
//!
//! #[nodetype]
//! async fn Record(input: String, memory: &Mutex<Vec<String>>) -> (String, ()) {
//!     memory.lock().await.push(input.clone());
//!     (input, ())
//! }
//!
//! #[nodetype]
//! pub async fn EchoAndExit(memory: &Mutex<Vec<String>>) {
//!     println!("You entered:");
//!     let inputs = memory.lock().await;
//!     for input in inputs.iter() {
//!         println!("{}", input);
//!     }
//! }
//!
//! graph! {
//!     digraph MemoryEcho {
//!         node[shape=box];
//!
//!         get_input[label="Get Input", type=GetInput, start=true];
//!         echo_and_exit[label="Echo Recorded\nInput and Exit", type=EchoAndExit];
//!         record[label="Record and Loop", type=Record, branch=matcher];
//!
//!         get_input -> record;
//!         record -> echo_and_exit [label=exit, value=exit];
//!         record -> get_input;
//!     }
//! }
//!
//! fn main() {
//!     MemoryEcho::run(());
//! }
//! ```

mod branchtracker;
mod dependencies;

pub use conflagrate_macros::{dependency, graph, nodetype};
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
