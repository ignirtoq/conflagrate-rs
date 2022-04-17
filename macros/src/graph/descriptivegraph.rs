use crate::graph::node::Nodes;
use std::collections::HashMap;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use dot_structures::{Attribute, Edge as GvEdge, EdgeTy, Graph as GvGraph, Id, Node as GvNode, Stmt, Vertex};

const NODE_TYPE_ATTR: &str = "type";
const NODE_BRANCH_ATTR: &str = "branch";
const NODE_START_ATTR: &str = "start";
const EDGE_VALUE_ATTR: &str = "value";

/// The parsed graph structure of the application.
///
/// A DescriptiveGraph contains the data extracted from parsing the raw string provided to the
/// `graph` macro.  It needs to be passed into the `ExecutableGraph` to be converted into Rust
/// source code that can be compiled and executed.
pub struct DescriptiveGraph {
    name: String,
    nodes: HashMap<String, Nodes>,
    start_node: String,
    source: String,
}
impl DescriptiveGraph {
    pub fn from(raw_source: &String) -> DescriptiveGraph {
        let gv_graph = match graphviz_rust::parse(&raw_source) {
            Ok(g) => g,
            Err(_) => panic!("unable to parse graph")
        };
        let mut graph = DescriptiveGraph::new(&get_graph_name(&gv_graph));
        graph.source = raw_source.clone();
        graph.process_graph(gv_graph);
        graph
    }

    fn new(name: &String) -> DescriptiveGraph {
        DescriptiveGraph {
            name: name.clone(),
            nodes: HashMap::<String, Nodes>::new(),
            start_node: String::new(),
            source: String::new(),
        }
    }

    fn add_node(&mut self, name: &String, nodetype: &String, branch: &String) {
        self.nodes.insert(name.clone(), Nodes::new_node(&name, &nodetype, &branch));
    }

    fn add_edge(&mut self, source: &String, destination: &String, value: &String) {
        match self.nodes.get_mut(source) {
            Some(node) => {
                node.add_destination(&value, &destination);
            },
            None => {}
        }
    }

    fn process_graph(&mut self, gv_graph: GvGraph) {
        match gv_graph {
            GvGraph::Graph {id: _, stmts, strict: _}
                | GvGraph::DiGraph {id: _, stmts, strict: _} => {
                for statement in stmts.iter() {
                    self.process_statement(statement);
                }
            }
        }
    }

    fn process_statement(&mut self, statement: &Stmt) {
        match statement {
            Stmt::Node(node) => self.process_node(node),
            Stmt::Edge(edge) => self.process_edge(edge),
            _ => {}
        }
    }

    fn process_node(&mut self, node: &GvNode) {
        match get_nodetype_from_gv_node(node) {
            Some(nodetype) => {
                let node_id = id_to_string(&node.id.0);
                let branch = get_branch_value_from_node_attributes(&node.attributes);
                self.add_node(&node_id, &nodetype, &branch);
                if is_start_node(&node) {
                    self.start_node = node_id;
                }
            },
            None => {}
        }
    }

    fn process_edge(&mut self, edge: &GvEdge) {
        match &edge.ty {
            EdgeTy::Pair(src, dest) => self.process_edge_pair(src, dest, &edge.attributes),
            EdgeTy::Chain(vertices) => self.process_edge_chain(vertices, &edge.attributes)
        }
    }

    fn process_edge_chain(&mut self, vertices: &Vec<Vertex>, attrs: &Vec<Attribute>) {
        for i in 1..=vertices.len()-1 {
            self.process_edge_pair(&vertices[i-1], &vertices[i], attrs);
        }
    }

    fn process_edge_pair(&mut self, src: &Vertex, dest: &Vertex, attrs: &Vec<Attribute>) {
        let matcher_value = get_match_value_from_edge_attributes(attrs);
        match (src, dest) {
            (Vertex::N(src_id), Vertex::N(dest_id)) =>
                self.add_edge(
                    &id_to_string(&src_id.0),
                    &id_to_string(&dest_id.0),
                    &matcher_value,
                ),
            _ => {}
        }
    }

    pub fn get_output_type(&self) -> TokenStream {
        let mut output_type: Option<Ident> = None;
        for val in self.nodes.values() {
            if val.is_terminating_node() {
                output_type = Some(val.get_nodetype_ident());
            }
        }
        match output_type {
            Some(ident) => quote!{<#ident as conflagrate::NodeType>::ReturnType},
            None => quote!{()}
        }
    }

    fn get_start_node(&self) -> Option<&Nodes> {
        self.nodes.get(&self.start_node)
    }

    pub fn get_start_node_name(&self) -> &String {
        match self.get_start_node() {
            Some(node) => node.get_name(),
            None => panic!("No starting node found!  Give one node the attribute 'start'.")
        }
    }

    pub fn get_start_node_nodetype(&self) -> TokenStream {
        match self.get_start_node() {
            Some(start_node) => {
                let start_node_nodetype = start_node.get_nodetype_ident();
                quote!{#start_node_nodetype}
            },
            None => panic!("No starting node found!  Give one node the attribute 'start'.")
        }
    }

    pub fn get_name(&self) -> Ident {
        format_ident!("{}", &self.name)
    }

    pub fn get_nodes(&self) -> &HashMap<String, Nodes> {
        &self.nodes
    }

    pub fn into_source(self) -> String {
        self.source
    }
}

fn get_graph_name(graph: &GvGraph) -> String {
    match graph {
        GvGraph::Graph {id, strict: _, stmts: _}
        | GvGraph::DiGraph {id, strict: _, stmts: _} => {
            let name = id_to_string(id);
            if !name.is_empty() { name } else { String::from("Graph") }
        }
    }
}

fn id_to_string(id: &Id) -> String {
    match id {
        Id::Html(val) | Id::Escaped(val) | Id::Plain(val) => val.clone(),
        Id::Anonymous(_) => String::new(),
    }
}

fn get_nodetype_from_gv_node(node: &GvNode) -> Option<String> {
    for attr in node.attributes.iter() {
        let attr_key = id_to_string(&attr.0);
        if attr_key == NODE_TYPE_ATTR {
            return Some(id_to_string(&attr.1))
        }
    }
    None
}

fn is_start_node(node: &GvNode) -> bool {
    for attr in node.attributes.iter() {
        let attr_key = id_to_string(&attr.0);
        if attr_key == NODE_START_ATTR { return true; }
    }
    false
}

fn get_branch_value_from_node_attributes(attrs: &Vec<Attribute>) -> String {
    for attr in attrs.iter() {
        let attr_key = id_to_string(&attr.0);
        if attr_key == NODE_BRANCH_ATTR {
            return id_to_string(&attr.1);
        }
    }
    String::new()
}

fn get_match_value_from_edge_attributes(attributes: &Vec<Attribute>) -> String {
    for attr in attributes.iter() {
        let attr_key = id_to_string(&attr.0);
        if attr_key == EDGE_VALUE_ATTR {
            return id_to_string(&attr.1)
        }
    }
    String::new()
}
