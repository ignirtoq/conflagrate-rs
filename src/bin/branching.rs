use conflagrate::{graph, nodetype};

#[nodetype]
pub fn Greeting() -> (String, ()) {
    let mut choice = String::new();
    println!("Pick a path (1 or 2):");
    std::io::stdin().read_line(&mut choice).unwrap();
    choice.truncate(choice.len() - 1);
    (choice, ())
}

#[nodetype(NONBLOCKING)]
pub fn Option1() {
    println!("You chose option 1");
}

#[nodetype(NONBLOCKING)]
pub fn Option2() {
    println!("You chose option 2");
}

#[nodetype(NONBLOCKING)]
pub fn DefaultOption() {
    println!("Whoops!  Unexpected input.");
}

graph!{

digraph {
    start[label="Greeting", type=Greeting, branch=matcher, start=true];
    option1[label="Choice 1", type=Option1];
    option2[label="Choice 2", type=Option2];
    default[label="Default Behavior", type=DefaultOption];
    start -> option1 [value=1];
    start -> option2 [value=2];
    start -> default;
}

}

fn main() {
    Graph::run(())
}
