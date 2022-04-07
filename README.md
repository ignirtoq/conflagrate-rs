🔥 Conflagrate
==============

Build applications from control flow graphs, rather than the other way around.

1. Define your application flow with a [Graphviz](https://www.graphviz.org/) diagram
2. Write the code for each node as a function
3. Run

Conflagrate is a framework for building applications structured as the 
control flow graphs they end up becoming anyway.

Build the pieces of your application as self-contained nodes.  Arrange and
rearrange control flow logic as your needs mature and change without having to
go back and rewrite the glue code connecting your components.

🔨 Build the Graph
------------------
Define your application control flow with a graph.
```dot
conflagrate::graph!{

    digraph MessageHandlerGraph {
        listen[label="Listen on a Socket for a Message", type=Listen, start=true];
        handle_message[label="Handle the Message", type=HandleMessage];

        listen -> handle_message;  // Handle the message
        listen -> listen;  // Listen for the next message
    }

}
```

💻 Implement the Nodes
----------------------
Write a function for each type of node in your application.
```rust
use conflagrate::nodetype;

#[nodetype]
async fn Listen(interface: &SocketInterface) -> String {
    interface.receive().await
}

#[nodetype]
async fn HandleMessage(message: String, logger: &Logger) {
    logger.log(message);
}
```

🚀 Run
------
Run the application.
```rust
fn main() {
    MessageHandlerGraph::run(());
}
```