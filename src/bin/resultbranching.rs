use conflagrate::{graph, nodetype};

#[nodetype]
pub fn GetInput() -> Result<(), ()> {
    let mut input = String::new();
    println!("Please type 'success':");
    std::io::stdin().read_line(&mut input).unwrap();
    input.truncate(input.len() - 1);
    match input.as_str() {
        "success" => Ok(()),
        _ => Err(()),
    }
}

#[nodetype]
pub fn Success() {
    println!("Huzzah! Successful!");
}

#[nodetype]
pub fn SuccessParallel() {
    println!("Result-based branching can run parallel nodes as well!");
}

#[nodetype]
pub fn Error() {
    println!("Oops! Unexpected input!")
}

graph!{

digraph {
    start[label="Get input", type=GetInput, branch=resultmatcher, start=true];
    success[label="Success", type=Success];
    success_parallel[label="Success", type=SuccessParallel];
    error[label="Error", type=Error];
    start -> success [value=ok];
    start -> success_parallel [value=ok];
    start -> error [value=err];
}

}

fn main() {
    Graph::run(())
}
