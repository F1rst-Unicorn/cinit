use std::collections::HashMap;
use std::collections::VecDeque;

use petgraph::graph::Graph;

use config::config::ProcessConfig;

/// Process information relevant for dependency resolution
/// via ongoing topological sorting
#[derive(Debug)]
pub struct ProcessNode {
    pub after_self: Vec<usize>,

    pub predecessor_count: usize,
}

impl ProcessNode {

    pub fn new() -> ProcessNode {
        ProcessNode {
            after_self: Vec::new(),
            predecessor_count: 0
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Cycle(usize),
    Duplicate(usize),
}


#[derive(Debug)]
pub struct DependencyManager {
    nodes: Vec<ProcessNode>,

    pub runnable: VecDeque<usize>,
}

impl DependencyManager {

    /// Return a newly constructed dependency manager
    ///
    /// If the config contains cyclic dependency the Err(index)
    /// contains the index of some program involved in the cycle
    pub fn with_nodes(config: &Vec<ProcessConfig>) -> Result<Self, Error> {
        let name_dict = DependencyManager::build_name_dict(config)?;
        let nodes = DependencyManager::build_dependencies(config, name_dict);
        let result = DependencyManager {
            runnable: DependencyManager::find_initial_runnables(&nodes),
            nodes,
        };

        result.check_for_cycles()?;
        Ok(result)
    }

    pub fn has_runnables(&self) -> bool {
        !self.runnable.is_empty()
    }

    pub fn pop_runnable(&mut self) -> Option<usize> {
        self.runnable.pop_back()
    }

    pub fn notify_process_finished(&mut self, process: usize) -> Vec<usize> {
        let mut result = Vec::new();
        for successor_index in self.nodes[process].after_self.clone() {
            let mut successor = &mut self.nodes[successor_index];
            successor.predecessor_count -= 1;
            if successor.predecessor_count == 0 {
                // no need to remove `process` from successor's dependencies
                self.runnable.push_back(successor_index);
                result.push(successor_index);
            }
        }
        result
    }

    fn find_initial_runnables(nodes: &Vec<ProcessNode>) -> VecDeque<usize> {
        let mut result = VecDeque::new();
        nodes.iter()
            .enumerate()
            .filter(|(_, process)| process.predecessor_count == 0)
            .map(|(i, _)| result.push_back(i))
            .for_each(drop);
        result
    }

    fn build_dependencies(config: &Vec<ProcessConfig>, name_dict: HashMap<String, usize>) -> Vec<ProcessNode> {
        let mut result = Vec::with_capacity(config.len());

        for _ in 0..config.len() {
            result.push(ProcessNode::new());
        }

        for process_config in config {
            let current_index = name_dict
                .get(&process_config.name)
                .expect("Invalid index in name_dict")
                .clone();
            {
                let mut current = result
                    .get_mut(current_index)
                    .expect("Invalid index in name_dict");
                for predecessor_name in &process_config.before {
                    let predecessor_index = name_dict
                        .get(predecessor_name)
                        .expect("Invalid index in name_dict")
                        .clone();
                    current.after_self.push(predecessor_index);
                }

                current.predecessor_count += process_config.after.len();
            }

            for predecessor_name in &process_config.before {
                let predecessor_index = name_dict
                    .get(predecessor_name)
                    .expect("Invalid index in name_dict")
                    .clone();
                let mut predecessor = result
                    .get_mut(predecessor_index)
                    .expect("Invalid index in name_dict");
                predecessor.predecessor_count += 1;
            }

            for predecessor in &process_config.after {
                let dependency_index = name_dict
                    .get(predecessor)
                    .expect("Invalid index in name_dict")
                    .clone();
                let mut dependency = result
                    .get_mut(dependency_index)
                    .expect("Invalid index in name_dict");
                dependency.after_self.push(current_index);
            }
        }
        result
    }

    fn check_for_cycles(&self) -> Result<(), Error> {
        let mut graph = Graph::<_, _>::new();
        let mut node_dict = HashMap::new();

        for (i, _) in (&self.nodes).iter().enumerate() {
            let node = graph.add_node(i);
            node_dict.insert(i, node);
        }

        for (i, node) in (&self.nodes).iter().enumerate() {
            for successor in &node.after_self {
                graph.add_edge(
                    node_dict.get(&i).unwrap().clone(),
                    node_dict.get(successor).unwrap().clone(),
                    0,
                );
            }
        }

        if let Err(cycle) = petgraph::algo::toposort(&graph, None) {
            let node_id = cycle.node_id();
            Err(Error::Cycle(graph.node_weight(node_id).unwrap().clone()))

        } else {
            Ok(())
        }
    }

    fn build_name_dict(descriptions: &Vec<ProcessConfig>) -> Result<HashMap<String, usize>, Error> {
        let mut result = HashMap::new();

        for (i, desc) in descriptions.into_iter().enumerate() {
            if result.contains_key(&desc.name) {
                return Err(Error::Duplicate(i));
            } else {
                result.insert(desc.name.to_owned(), i);
            }
        }

        Ok(result)
    }
}
