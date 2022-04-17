use std::collections::HashMap;
use proc_macro2::Ident;
use quote::format_ident;

const NODE_BRANCH_MATCHER_VAL: &'static str = "matcher";
const NODE_BRANCH_RESULT_MATCHER_VAL: &'static str = "resultmatcher";

const RESULT_MATCHER_OK_VAL: &'static str = "ok";
const RESULT_MATCHER_ERR_VAL: &'static str = "err";

#[derive(Clone)]
pub struct Node {
    name: String,
    nodetype: String,
    destinations: Vec<String>,
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

    fn get_destinations(&self) -> Vec<String> {
        self.destinations.clone()
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_nodetype(&self) -> &String {
        &self.nodetype
    }

    fn is_terminating_node(&self) -> bool {
        self.destinations.is_empty()
    }
}

#[derive(Clone)]
pub struct MatcherNode {
    name: String,
    nodetype: String,
    destinations: HashMap<String, String>,
}
impl MatcherNode {
    fn new(name: &String, nodetype: &String) -> MatcherNode {
        MatcherNode {
            name: name.clone(),
            nodetype: nodetype.clone(),
            destinations: HashMap::<String, String>::new(),
        }
    }

    fn add_destination(&mut self, value: &String, destination: &String) {
        self.destinations.insert(value.clone(), destination.clone());
    }

    fn get_destinations(&self) -> HashMap<String, String> {
        self.destinations.clone()
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_nodetype(&self) -> &String {
        &self.nodetype
    }

    fn is_terminating_node(&self) -> bool {
        self.destinations.is_empty()
    }
}

#[derive(Clone)]
pub struct ResultDestinations {
    ok: Vec<String>,
    err: Vec<String>,
}
impl ResultDestinations {
    pub fn new() -> Self {
        Self {
            ok: Vec::<String>::new(),
            err: Vec::<String>::new(),
        }
    }

    pub fn get_ok_nodes(&self) -> Vec<String> {
        self.ok.clone()
    }

    pub fn get_err_nodes(&self) -> Vec<String> {
        self.err.clone()
    }

    pub fn len(&self) -> usize {
        self.err.len() + self.ok.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone)]
pub struct ResultMatcherNode {
    name: String,
    nodetype: String,
    destinations: ResultDestinations,
}
impl ResultMatcherNode {
    fn new(name: &String, nodetype: &String) -> Self {
        Self {
            name: name.clone(),
            nodetype: nodetype.clone(),
            destinations: ResultDestinations::new(),
        }
    }

    fn add_destination(&mut self, value: &String, destination: &String) {
        let value_lower = value.to_lowercase();
        match value_lower.as_str() {
            RESULT_MATCHER_OK_VAL => self.destinations.ok.push(destination.clone()),
            RESULT_MATCHER_ERR_VAL => self.destinations.err.push(destination.clone()),
            _ => panic!("Result matcher node only supports ok and err edge values")
        }
    }

    fn get_destinations(&self) -> ResultDestinations {
        self.destinations.clone()
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_nodetype(&self) -> &String {
        &self.nodetype
    }

    fn is_terminating_node(&self) -> bool {
        self.destinations.ok.is_empty() && self.destinations.err.is_empty()
    }
}

pub enum Branches {
    Parallel(Vec<String>),
    Match(HashMap<String, String>),
    ResultMatch(ResultDestinations),
}

#[derive(Clone)]
pub enum Nodes {
    Node(Node),
    MatcherNode(MatcherNode),
    ResultMatcherNode(ResultMatcherNode),
}
impl Nodes {
    pub fn is_terminating_node(&self) -> bool {
        match self {
            Self::Node(node) => node.is_terminating_node(),
            Self::MatcherNode(node) => node.is_terminating_node(),
            Self::ResultMatcherNode(node) => node.is_terminating_node(),
        }
    }

    pub fn node_returns_matcher_value(&self) -> bool {
        match self {
            Self::Node(_) => false,
            Self::MatcherNode(_) => true,
            Self::ResultMatcherNode(_) => false,
        }
    }

    pub fn add_destination(&mut self, value: &String, destination: &String) {
        match self {
            Self::Node(node) => node.add_destination(destination),
            Self::MatcherNode(node) => node.add_destination(value, destination),
            Self::ResultMatcherNode(node) => node.add_destination(value, destination),
        }
    }

    pub fn get_nodetype_ident(&self) -> Ident {
        match self {
            Self::Node(node) => format_ident!("{}", node.get_nodetype()),
            Self::MatcherNode(node) => format_ident!("{}", node.get_nodetype()),
            Self::ResultMatcherNode(node) => format_ident!("{}", node.get_nodetype()),
        }
    }

    pub fn new_node(name: &String, nodetype: &String, branch: &String) -> Self {
        match branch.as_str() {
            NODE_BRANCH_MATCHER_VAL => Self::MatcherNode(MatcherNode::new(&name, &nodetype)),
            NODE_BRANCH_RESULT_MATCHER_VAL => Self::ResultMatcherNode(
                ResultMatcherNode::new(&name, &nodetype)
            ),
            _ => Self::Node(Node::new(&name, &nodetype))
        }
    }

    pub fn get_name(&self) -> &String {
        match self {
            Self::Node(node) => node.get_name(),
            Self::MatcherNode(node) => node.get_name(),
            Self::ResultMatcherNode(node) => node.get_name(),
        }
    }

    pub fn get_destinations(&self) -> Branches {
        match self {
            Self::Node(node) => Branches::Parallel(node.get_destinations()),
            Self::MatcherNode(node) => Branches::Match(node.get_destinations()),
            Self::ResultMatcherNode(node) => Branches::ResultMatch(node.get_destinations())
        }
    }
}