use std::collections::HashMap;
use std::collections::VecDeque;

use petgraph::graph::Graph;

use config::config::ProcessConfig;

/// Process information relevant for dependency resolution
/// via ongoing topological sorting
#[derive(Debug, PartialEq)]
pub struct ProcessNode {
    pub after_self: Vec<usize>,

    pub predecessor_count: usize,
}

impl ProcessNode {
    pub fn new() -> ProcessNode {
        ProcessNode {
            after_self: Vec::new(),
            predecessor_count: 0,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Cycle(usize),
    Duplicate(usize),
}

#[derive(Debug, PartialEq)]
pub struct DependencyManager {
    nodes: HashMap<usize, ProcessNode>,

    runnable: VecDeque<usize>,
}

impl DependencyManager {
    /// Return a newly constructed dependency manager
    ///
    /// If the config contains cyclic dependency the Err(index)
    /// contains the index of some program involved in the cycle
    pub fn with_nodes(config: &Vec<(usize, ProcessConfig)>) -> Result<Self, Error> {
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

    pub fn notify_process_finished(&mut self, process: usize) {
        for successor_index in self.nodes[&process].after_self.clone() {
            let mut successor = self.nodes.get_mut(&successor_index).expect("Invalid index");
            successor.predecessor_count -= 1;
            if successor.predecessor_count == 0 {
                // no need to remove `process` from successor's dependencies
                self.runnable.push_back(successor_index);
            }
        }
    }

    fn find_initial_runnables(nodes: &HashMap<usize, ProcessNode>) -> VecDeque<usize> {
        let mut result = VecDeque::new();
        nodes
            .iter()
            .filter(|(_, process)| process.predecessor_count == 0)
            .map(|(i, _)| result.push_back(*i))
            .for_each(drop);
        result
    }

    fn build_dependencies(
        config: &Vec<(usize, ProcessConfig)>,
        name_dict: HashMap<String, usize>,
    ) -> HashMap<usize, ProcessNode> {
        let mut result = HashMap::with_capacity(config.len());

        for (k, _) in config.iter() {
            result.insert(*k, ProcessNode::new());
        }

        for (current_index, current_config) in (&config).iter() {
            {
                let mut current = result
                    .get_mut(current_index)
                    .expect("Invalid index in name_dict");
                for successor_name in &current_config.before {
                    let successor_index = name_dict
                        .get(successor_name)
                        .expect("Invalid index in name_dict")
                        .clone();
                    current.after_self.push(successor_index);
                }

                current.predecessor_count += current_config.after.len();
            }

            for successor_name in &current_config.before {
                let successor_index = name_dict
                    .get(successor_name)
                    .expect("Invalid index in name_dict")
                    .clone();
                let mut successor = result
                    .get_mut(&successor_index)
                    .expect("Invalid index in name_dict");
                successor.predecessor_count += 1;
            }

            for predecessor_name in &current_config.after {
                let predecessor_index = name_dict
                    .get(predecessor_name)
                    .expect("Invalid index in name_dict")
                    .clone();
                let mut predecessor = result
                    .get_mut(&predecessor_index)
                    .expect("Invalid index in name_dict");
                predecessor.after_self.push(*current_index);
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

        for (i, node) in (&self.nodes).iter() {
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

    fn build_name_dict(
        descriptions: &Vec<(usize, ProcessConfig)>,
    ) -> Result<HashMap<String, usize>, Error> {
        let mut result = HashMap::new();

        for (i, desc) in descriptions.into_iter() {
            if result.contains_key(&desc.name) {
                return Err(Error::Duplicate(*i));
            } else {
                result.insert(desc.name.to_owned(), *i);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::config::{ProcessConfig, ProcessType};

    #[test]
    pub fn single_runnable_process() {
        let config = vec![(0, ProcessConfig::new("first", vec![], vec![]))];

        let mut uut =
            DependencyManager::with_nodes(&config).expect("Failed to create dependency manager");

        assert!(uut.has_runnables());
        assert_eq!(Some(0), uut.pop_runnable());
        assert!(!uut.has_runnables());
        assert_eq!(None, uut.pop_runnable());
    }

    #[test]
    pub fn cyclic_dependency() {
        let config = vec![
            (0, ProcessConfig::new("first", vec!["second"], vec![])),
            (1, ProcessConfig::new("second", vec!["first"], vec![])),
        ];

        let uut = DependencyManager::with_nodes(&config);

        assert!(uut.is_err());
        assert!(Err(Error::Cycle(0)) == uut || Err(Error::Cycle(1)) == uut);
    }

    #[test]
    pub fn duplicate_name() {
        let config = vec![
            (0, ProcessConfig::new("first", vec![], vec![])),
            (1, ProcessConfig::new("first", vec![], vec![])),
        ];

        let uut = DependencyManager::with_nodes(&config);

        assert!(uut.is_err());
        assert!(Err(Error::Duplicate(0)) == uut || Err(Error::Duplicate(1)) == uut);
    }

    #[test]
    pub fn dependants_are_marked_runnable() {
        let config = vec![
            (0, ProcessConfig::new("first", vec!["second"], vec![])),
            (1, ProcessConfig::new("second", vec![], vec![])),
        ];
        let mut uut =
            DependencyManager::with_nodes(&config).expect("Failed to create dependency manager");
        uut.pop_runnable().expect("Assumption broken");
        uut.notify_process_finished(0);

        assert!(uut.has_runnables());
        assert_eq!(Some(1), uut.pop_runnable());
        assert!(!uut.has_runnables());
        assert_eq!(None, uut.pop_runnable());
    }

    #[test]
    pub fn have_two_dependencies() {
        let config = vec![
            (0, ProcessConfig::new("first", vec![], vec![])),
            (1, ProcessConfig::new("second", vec!["third"], vec![])),
            (2, ProcessConfig::new("third", vec![], vec!["first"])),
        ];
        let mut uut =
            DependencyManager::with_nodes(&config).expect("Failed to create dependency manager");
        uut.pop_runnable().expect("Assumption broken");
        uut.pop_runnable().expect("Assumption broken");
        assert!(!uut.has_runnables());
        uut.notify_process_finished(0);
        assert!(!uut.has_runnables());
        uut.notify_process_finished(1);

        assert!(uut.has_runnables());
        assert_eq!(Some(2), uut.pop_runnable());
        assert!(!uut.has_runnables());
        assert_eq!(None, uut.pop_runnable());
    }

    impl ProcessConfig {
        pub fn new(name: &str, before: Vec<&str>, after: Vec<&str>) -> ProcessConfig {
            ProcessConfig {
                name: name.to_string(),
                path: "".to_string(),
                args: vec![],
                workdir: None,
                process_type: ProcessType::Oneshot,
                uid: None,
                gid: None,
                user: None,
                group: None,
                before: before.iter().map(<&str>::to_string).collect(),
                after: after.iter().map(<&str>::to_string).collect(),
                emulate_pty: false,
                capabilities: vec![],
                env: vec![],
            }
        }
    }
}
