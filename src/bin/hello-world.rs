#[conflagrate::nodetype]
fn StartNodeType() -> String {
    let mut name = String::new();
    println!("Hello, what is your name?");
    std::io::stdin().read_line(&mut name).unwrap();
    name.truncate(name.len() - 1);
    name
}

#[conflagrate::nodetype(NONBLOCKING)]
fn GreetingNodeType(name: String) {
    println!("Welcome {}!", name)
}

conflagrate::graph!{

strict digraph {
  start[label="Get Name", type=StartNodeType, start=true]
  welcome[label="Greeting", type=GreetingNodeType]
  start->welcome
}

}

fn main() {
    Graph::run(())
}
