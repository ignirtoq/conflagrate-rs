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
