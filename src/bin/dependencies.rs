use tokio::sync::Mutex;
use conflagrate::{dependency, graph, nodetype};

#[dependency]
async fn memory() -> Mutex<Vec<String>> {
    Mutex::new(Vec::<String>::new())
}

#[nodetype]
pub fn GetInput() -> String {
    let mut input = String::new();
    println!("Type any input ('exit' to exit):");
    std::io::stdin().read_line(&mut input).unwrap();
    input.truncate(input.len() - 1);
    input
}

#[nodetype]
async fn Record(input: String, memory: &Mutex<Vec<String>>) -> (String, ()) {
    memory.lock().await.push(input.clone());
    (input, ())
}

#[nodetype]
pub async fn EchoAndExit(memory: &Mutex<Vec<String>>) {
    println!("You entered:");
    let inputs = memory.lock().await;
    for input in inputs.iter() {
        println!("{}", input);
    }
}

graph! {
    digraph MemoryEcho {
        node[shape=box];

        get_input[label="Get Input", type=GetInput, start=true];
        echo_and_exit[label="Echo Recorded\nInput and Exit", type=EchoAndExit];
        record[label="Record and Loop", type=Record, branch=matcher];

        get_input -> record;
        record -> echo_and_exit [label=exit, value=exit];
        record -> get_input;
    }
}

fn main() {
    MemoryEcho::run(());
}
