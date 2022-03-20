use std::collections::HashMap;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

const DEFAULT_MATCH_VALUE: &str = "";
const NODE_BRANCH_MATCHER_VAL: &str = "matcher";

fn branchtracker_add_branch() -> TokenStream {
    quote! {
        branchtracker.lock().await.add_branch();
    }
}

fn branchtracker_remove_branch() -> TokenStream {
    quote! {
        branchtracker.lock().await.remove_branch(output);
    }
}

trait BaseNode {
    fn get_name(&self) -> &String;

    fn get_nodetype(&self) -> &String;

    fn is_terminating_node(&self) -> bool;

    fn create_tail(&self) -> TokenStream;

    fn create_spawn_block(next_node_name: &String) -> TokenStream {
        let execute_node = format_ident!("execute_{}", next_node_name);
        quote! {
            {
                let bclone = branchtracker.clone();
                let oclone = output.clone();
                tokio::spawn(async move {
                    Self::#execute_node(bclone, oclone).await;
                });
            }
        }
    }

    fn create_execute_fn(&self, graph_output_type: TokenStream) -> TokenStream;

    fn get_execute_ident(&self) -> Ident {
        format_ident!("execute_{}", self.get_name())
    }

    fn get_nodetype_ident(&self) -> Ident {
        format_ident!("{}", self.get_nodetype())
    }

    fn create_spawn_blocks(&self) -> TokenStream {
        if self.is_terminating_node() {
            branchtracker_remove_branch()
        } else {
            self.create_tail()
        }
    }
}

pub struct Node {
    name: String,
    nodetype: String,
    destinations: Vec<String>,
}
impl BaseNode for Node {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_nodetype(&self) -> &String {
        &self.nodetype
    }

    fn is_terminating_node(&self) -> bool {
        self.destinations.is_empty()
    }

    fn create_tail(&self) -> TokenStream {
        let mut out = TokenStream::new();
        let mut needs_add_branch = false;
        for dest in self.destinations.iter() {
            if needs_add_branch {
                out.extend(branchtracker_add_branch())
            }
            out.extend(Self::create_spawn_block(&dest));
            needs_add_branch = true;
        }
        out
    }

    fn create_execute_fn(&self, graph_output_type: TokenStream) -> TokenStream {
        let node_type = self.get_nodetype_ident();
        let execute_node = self.get_execute_ident();
        let spawn_blocks = self.create_spawn_blocks();
        quote!{
            async fn #execute_node(
                branchtracker: std::sync::Arc<tokio::sync::Mutex<conflagrate::BranchTracker<#graph_output_type>>>,
                node_args: <#node_type as conflagrate::NodeType>::Args
            ) {
                let output = <#node_type as conflagrate::NodeType>::run(node_args).await;
                #spawn_blocks
            }
        }
    }
}
impl Node {
    fn new(name: &String, nodetype: &String) -> Node {
        Node {
            name: name.clone(),
            destinations: Vec::<String>::new(),
            nodetype: nodetype.clone(),
        }
    }

    fn add_destination(&mut self, destination: &String) {
        self.destinations.push(destination.clone());
    }
}

pub struct MatcherNode {
    name: String,
    nodetype: String,
    destinations: HashMap<String, String>,
}
impl BaseNode for MatcherNode {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_nodetype(&self) -> &String {
        &self.nodetype
    }

    fn is_terminating_node(&self) -> bool {
        self.destinations.is_empty()
    }

    fn create_tail(&self) -> TokenStream {
        let match_blocks = self.create_match_blocks();
        quote! {
            match value.as_str() {
                #match_blocks
            }
        }
    }

    fn create_execute_fn(&self, graph_output_type: TokenStream) -> TokenStream {
        let node_type = self.get_nodetype_ident();
        let execute_node = self.get_execute_ident();
        let spawn_blocks = self.create_spawn_blocks();
        quote!{
            async fn #execute_node(
                branchtracker: std::sync::Arc<tokio::sync::Mutex<conflagrate::BranchTracker<#graph_output_type>>>,
                node_args: <#node_type as conflagrate::NodeType>::Args
            ) {
                let (value, output) = <#node_type as conflagrate::NodeType>::run(node_args).await;
                #spawn_blocks
            }
        }
    }
}
impl MatcherNode {
    fn new(name: &String, nodetype: &String) -> MatcherNode {
        MatcherNode {
            name: name.clone(),
            nodetype: nodetype.clone(),
            destinations: HashMap::<String, String>::new(),
        }
    }

    fn create_match_block(value: &String, spawn_block: &TokenStream) -> TokenStream {
        quote!{
            #value => #spawn_block,
        }
    }

    fn create_match_blocks(&self) -> TokenStream {
        let mut out = TokenStream::new();
        for (value, node_name) in self.destinations.iter() {
            if value == DEFAULT_MATCH_VALUE {
                continue;
            } else {
                out.extend(Self::create_match_block(
                    value,
                    &Self::create_spawn_block(node_name)
                ))
            }
        }
        out.extend(self.create_default_match());
        out
    }

    fn create_default_match(&self) -> TokenStream {
        match self.destinations.get("") {
            Some(node_name) => {
                let spawn_block = Self::create_spawn_block(node_name);
                quote!{
                    _ => #spawn_block
                }
            },
            None => {
                let remove_branch = branchtracker_remove_branch();
                quote!{
                    _ => #remove_branch
                }
            }
        }
    }

    fn add_destination(&mut self, value: &String, destination: &String) {
        self.destinations.insert(value.clone(), destination.clone());
    }
}

pub enum Nodes {
    Node(Node),
    MatcherNode(MatcherNode),
}
impl Nodes {
    pub fn is_terminating_node(&self) -> bool {
        match self {
            Self::Node(node) => node.is_terminating_node(),
            Self::MatcherNode(node) => node.is_terminating_node()
        }
    }

    pub fn add_destination(&mut self, value: &String, destination: &String) {
        match self {
            Self::Node(node) => node.add_destination(destination),
            Self::MatcherNode(node) => node.add_destination(value, destination)
        }
    }

    pub fn get_nodetype_ident(&self) -> Ident {
        match self {
            Self::Node(node) => node.get_nodetype_ident(),
            Self::MatcherNode(node) => node.get_nodetype_ident()
        }
    }

    pub fn get_execute_ident(&self) -> Ident {
        match self {
            Self::Node(node) => node.get_execute_ident(),
            Self::MatcherNode(node) => node.get_execute_ident()
        }
    }

    pub fn create_execute_fn(&self, graph_output_type: TokenStream) -> TokenStream {
        match self {
            Self::Node(node) => node.create_execute_fn(graph_output_type),
            Self::MatcherNode(node) => node.create_execute_fn(graph_output_type)
        }
    }

    pub fn new_node(name: &String, nodetype: &String, branch: &String) -> Self {
        match branch.as_str() {
            NODE_BRANCH_MATCHER_VAL => Self::MatcherNode(MatcherNode::new(&name, &nodetype)),
            _ => Self::Node(Node::new(&name, &nodetype))
        }
    }
}