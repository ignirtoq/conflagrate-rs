use conflagrate::{graph, nodetype};

#[nodetype]
fn StartNodeType() -> String {
    let mut name = String::new();
    println!("Hello, what is your name?");
    std::io::stdin().read_line(&mut name).unwrap();
    name.truncate(name.len() - 1);
    name
}

#[nodetype(NONBLOCKING)]
fn FirstGreeting(name: String) {
    println!("Welcome {}!", name);
}

#[nodetype(NONBLOCKING)]
async fn SecondGreeting(_: String) {
    println!("This is a control flow graph based application.");
}

graph!{
strict digraph {
  start[label="Get Name", type=StartNodeType, start=true];
  welcome1[label="Greeting", type=FirstGreeting];
  welcome2[label="Greeting", type=SecondGreeting];
  start->welcome1;
  start->welcome2;
}
}

fn main() {
    Graph::run(())
}
