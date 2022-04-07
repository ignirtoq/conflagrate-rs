use conflagrate::{dependency, graph, nodetype};

struct Config {
    name: String,
    port: u16,
}

#[dependency]
async fn config() -> Config {
    Config { name: String::from("Dan"), port: 25535 }
}

#[dependency]
async fn name(config: &Config) -> String {
    config.name.clone()
}

#[dependency]
async fn port(config: &Config) -> u16 {
    config.port
}

#[nodetype(NONBLOCKING)]
pub fn Start() -> String {
    String::from("Hello")
}

#[nodetype(NONBLOCKING)]
fn PrintName(greeting: String, name: &String) {
    println!("{}, {}!", greeting, name);
}

#[nodetype(NONBLOCKING)]
fn PrintPort(greeting: String, port: &u16) {
    println!("{}! Coming to you live from port {}!", greeting, port);
}

#[nodetype(NONBLOCKING)]
pub fn End() {}

graph!{

    digraph MyGraph {
        start[label="Start!", type=Start, start=true]
        printName[label="Print name", type=PrintName];
        printPort[label="Print port?", type=PrintPort];
        end[label="The end", type=End];
        start -> printName;
        start -> printPort;
        printName -> end;
        printPort -> end;
    }

}


fn main() {
    MyGraph::run(());
}