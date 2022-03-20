pub mod branchtracker;

pub use conflagrate_macros::{graph, nodetype};
pub use branchtracker::BranchTracker;

#[async_trait::async_trait]
pub trait NodeType {
    type Args: Clone;
    type ReturnType;
    async fn run(args: Self::Args) -> Self::ReturnType;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        println!("it works!")
    }
}
