use std::collections::HashMap;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use crate::graph::node::{Branches, Nodes, ResultDestinations};
use crate::graph::task::MatchCase::{DefaultCase, RegularCase};

const DEFAULT_MATCH_VALUE: &str = "";

/// A task is a body of code executed as one "unit" in the asynchronous runtime.
///
/// In conflagrate, the role of a task is to execute one (or more) nodes and then spawn the next
/// task(s) in the graph.
///
/// In most cases a task will execute one node, however for optimization it may execute several
/// nodes if they occur in a single, linear progression with no branching (e.g. A->B->C).
pub struct Task {
    name: TaskName,
    invocation: Invocation,
    spawn: Spawn,
    graph_output_type: TokenStream,
}
impl Task {
    pub fn from_nodes(nodes: &[Nodes], graph_output_type: &TokenStream) -> Self {
        Self {
            name: TaskName::from(nodes.get(0).unwrap().get_name()),
            invocation: Invocation(Vec::from(nodes)),
            spawn: Spawn::from_nodes(nodes),
            graph_output_type: graph_output_type.clone()
        }
    }
}
impl ToTokens for Task {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let execute_node = &self.name;
        let graph_output_type = &self.graph_output_type;
        let first_node_type = self.invocation.get_nodetype();
        let invocation = &self.invocation;
        let spawn = &self.spawn;

        tokens.extend(quote! {
            #[async_recursion::async_recursion]
            async fn #execute_node(
                branchtracker: std::sync::Arc<tokio::sync::Mutex<conflagrate::BranchTracker<#graph_output_type>>>,
                node_args: <#first_node_type as conflagrate::NodeType>::Args,
                deps: std::sync::Arc<conflagrate::DependencyCache>
            ) {
                #invocation
                #spawn
            }
        })
    }
}

/// Name of the function executing the task, typically "execute_{nodename}" of the first node in
/// the task.
pub struct TaskName(Ident);
impl From<&String> for TaskName {
    fn from(name: &String) -> Self {
        Self(format_ident!("execute_{}", name))
    }
}
impl ToTokens for TaskName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let execute_task = &self.0;
        tokens.extend(quote!{#execute_task})
    }
}

/// How the nodes executed by this task are invoked.
///
/// Examples:
///
/// The trivial case of a single parallel-branch node invocation:
/// ```no_compile
/// let output = <{node_type} as conflagrate::NodeType>::run(node_args, &deps).await;
/// ```
///
/// Case of a single match-branch node invocation:
/// ```no_compile
/// let (value, output) = <{node_type} as conflagrate::NodeType>::run(node_args, &deps).await;
/// ```
/// This separates the matching `value` from the remainder `output` that is passed to the next node.
///
/// Case of multiple node invocations culminating in a parallel-branch node:
/// ```no_compile
/// let output = <{node_type2} as conflagrate::NodeType>::run(
///     <{node_type1} as conflagrate::NodeType>::run(node_args, &deps).await,
///     &deps
/// ).await;
/// ```
struct Invocation(Vec<Nodes>);
impl Invocation {
    fn get_nodetype(&self) -> Ident {
        self.0.get(0).unwrap().get_nodetype_ident()
    }

    fn node_to_invocation(node: &Nodes, node_args: &TokenStream) -> TokenStream {
        let node_type = node.get_nodetype_ident();
        quote! {
            <#node_type as conflagrate::NodeType>::run(#node_args, &deps).await
        }
    }

    fn get_nested_node_invocation(&self) -> TokenStream {
        let mut node_args = quote!{node_args};
        for node in self.0.iter() {
            node_args = Self::node_to_invocation(node, &node_args);
        }
        node_args
    }

    fn get_return_capture_args(&self) -> TokenStream {
        if self.0.last().unwrap().node_returns_matcher_value() {
            quote!{(value, output)}
        } else {
            quote!{output}
        }
    }
}
impl ToTokens for Invocation {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let nested_invocation = self.get_nested_node_invocation();
        let return_capture = self.get_return_capture_args();
        tokens.extend(quote! {
            let #return_capture = #nested_invocation;
        })
    }
}

enum Spawn {
    SpawnNone,
    SpawnParallel(SpawnParallel),
    SpawnMatch(SpawnMatch),
    SpawnResultMatch(SpawnResultMatch),
}
impl Spawn {
    fn from_nodes(nodes: &[Nodes]) -> Self {
        if nodes.len() == 0 {
            return Self::SpawnNone
        }
        let final_node = nodes.last().unwrap();
        match final_node.get_destinations() {
            Branches::Parallel(branches) => {
                if branches.is_empty() {
                    return Self::SpawnNone;
                }
                Spawn::SpawnParallel(SpawnParallel(convert_vec_string_to_vec_task_name(&branches)))
            },
            Branches::Match(branch_map) => {
                if branch_map.is_empty() {
                    return Self::SpawnNone;
                }

                let mut task_map = HashMap::<String, TaskName>::with_capacity(branch_map.len());
                for mapping in branch_map {
                    task_map.insert(mapping.0, TaskName::from(&mapping.1));
                }
                Spawn::SpawnMatch(SpawnMatch::from(task_map))
            },
            Branches::ResultMatch(destinations) => {
                if destinations.is_empty() {
                    return Self::SpawnNone;
                }
                Spawn::SpawnResultMatch(SpawnResultMatch(destinations))
            },
        }
    }
}
impl ToTokens for Spawn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Spawn::SpawnNone => {
                tokens.extend(branchtracker_remove_branch());
            },
            Spawn::SpawnParallel(spawn) => spawn.to_tokens(tokens),
            Spawn::SpawnMatch(spawn) => spawn.to_tokens(tokens),
            Spawn::SpawnResultMatch(spawn) => spawn.to_tokens(tokens),
        }
    }
}

/// Loop over each branch and create a spawn block, adding branches to the branch-tracker as needed.
///
/// SpawnParallel will create a codeblock that looks like the following:
/// ```no_compile
/// {
///     let bclone = branchtracker.clone();
///     let oclone = output.clone();
///     let dclone = std::sync::Arc::clone(&deps);
///     tokio::spawn(async move {
///         Self::execute_next_node1(bclone, oclone, dclone).await;
///     });
/// }
/// branchtracker.lock().await.add_branch();
/// {
///     let bclone = branchtracker.clone();
///     let oclone = output.clone();
///     let dclone = std::sync::Arc::clone(&deps);
///     tokio::spawn(async move {
///         Self::execute_next_node2(bclone, oclone, dclone).await;
///     });
/// }
/// // ...
/// ```
struct SpawnParallel(Vec<TaskName>);
impl ToTokens for SpawnParallel {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut needs_add_branch = false;
        let last_index = self.0.len() - 1;
        for (index, next_task) in self.0.iter().enumerate() {
            if needs_add_branch {
                tokens.extend(branchtracker_add_branch())
            }
            tokens.extend(create_spawn_block(&next_task, index == last_index));
            needs_add_branch = true;
        }
    }
}

/// Create a match block on the first return value of the executed node.
///
/// SpawnMatch will create a code block that looks like the following:
/// ```no_compile
/// match value.as_str() {
///     "value1" => {
///         let bclone = branchtracker.clone();
///         let oclone = output.clone();
///         let dclone = std::sync::Arc::clone(&deps);
///         tokio::spawn(async move {
///             Self::execute_next_node1(bclone, oclone, dclone).await;
///         });
///     },
///     "value2" => {
///         let bclone = branchtracker.clone();
///         let oclone = output.clone();
///         let dclone = std::sync::Arc::clone(&deps);
///         tokio::spawn(async move {
///             Self::execute_next_node2(bclone, oclone, dclone).await;
///         });
///     },
///     _ => {
///         let bclone = branchtracker.clone();
///         let oclone = output.clone();
///         let dclone = std::sync::Arc::clone(&deps);
///         tokio::spawn(async move {
///             Self::execute_next_node_default(bclone, oclone, dclone).await;
///         });
///     },
/// }
/// ```
///
/// In the case that no default branch is provided, the matcher node must output the same type as
/// the graph, and the default case will invoke the `remove_branch()` method of the branch tracker:
/// ```no_compile
/// match value.as_str() {
///     // ...
///     _ => {
///         branchtracker.lock().await.remove_branch(output);
///     }
/// }
/// ```
struct SpawnMatch(Vec<MatchCase>);
impl From<HashMap<String, TaskName>> for SpawnMatch {
    fn from(map: HashMap<String, TaskName>) -> Self {
        let mut match_cases = Vec::<MatchCase>::new();
        let mut default: MatchCase = MatchCase::NoDefault;
        for case in map {
            let case = MatchCase::from(case);
            if let MatchCase::DefaultCase(_) = case {
                default = case;
            } else {
                match_cases.push(case);
            }
        }
        match_cases.push(default);
        Self(match_cases)
    }
}
impl ToTokens for SpawnMatch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let cases = &self.0;
        tokens.extend(quote! {
            match value.as_str() {
                #(#cases)*
            }
        })
    }
}

enum MatchCase {
    RegularCase(String, TaskName),
    DefaultCase(TaskName),
    NoDefault,
}
impl From<(String, TaskName)> for MatchCase {
    fn from((value, task_name): (String, TaskName)) -> Self {
        match value.as_str() {
            DEFAULT_MATCH_VALUE => DefaultCase(task_name),
            _ => RegularCase(value, task_name),
        }
    }
}
impl ToTokens for MatchCase {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::RegularCase(value, task_name) => {
                let spawn_block = create_spawn_block(task_name, true);
                tokens.extend(quote! {
                    #value => #spawn_block,
                });
            },
            Self::DefaultCase(task_name) => {
                let spawn_block = create_spawn_block(task_name, true);
                tokens.extend(quote! {
                    _ => #spawn_block,
                });
            },
            Self::NoDefault => {
                let remove_branch_line = branchtracker_remove_branch();
                tokens.extend(quote! {
                    _ => {
                        #remove_branch_line
                    },
                });
            },
        }
    }
}

struct SpawnResultMatch(ResultDestinations);
impl SpawnResultMatch {
    fn destinations_to_blocks(destinations: &Vec<String>) -> TokenStream {
        if destinations.is_empty() {
            let remove_branch_line = branchtracker_remove_branch();
            quote! {
                {
                    #remove_branch_line
                }
            }
        } else {
            let spawn_parallel = SpawnParallel(convert_vec_string_to_vec_task_name(destinations));
            quote! {
               {
                   #spawn_parallel
               }
            }
        }
    }

    fn get_err_block(&self) -> TokenStream {
        Self::destinations_to_blocks(&self.0.get_err_nodes())
    }

    fn get_ok_block(&self) -> TokenStream {
        Self::destinations_to_blocks(&self.0.get_ok_nodes())
    }
}
impl ToTokens for SpawnResultMatch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ok_block = self.get_ok_block();
        let err_block = self.get_err_block();
        tokens.extend(quote!{
            match output {
                Ok(output) => #ok_block,
                Err(output) => #err_block,
            }
        })
    }
}

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

fn create_spawn_block(next_task_name: &TaskName, owns_args: bool) -> TokenStream {
    let branchtracker = if owns_args {quote! {branchtracker}} else {quote! {branchtracker.clone()}};
    let output = if owns_args {quote! {output}} else {quote! {output.clone()}};
    let deps = if owns_args {quote! {deps}} else {quote! {std::sync::Arc::clone(&deps)}};
    quote! {
        {
            let branchtracker = #branchtracker;
            let output = #output;
            let deps = #deps;
            tokio::spawn(async move {
                Self::#next_task_name(branchtracker, output, deps).await;
            });
        }
    }
}

fn convert_vec_string_to_vec_task_name(nodes: &Vec<String>) -> Vec<TaskName> {
    let mut task_names = Vec::<TaskName>::with_capacity(nodes.len());
    for node in nodes {
        task_names.push(TaskName::from(node))
    }
    task_names
}
