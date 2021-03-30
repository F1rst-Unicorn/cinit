/*  cinit: process initialisation program for containers
 *  Copyright (C) 2019 The cinit developers
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

//! Manage and evolve the dependency graph at runtime
//!
//! This data structure performs a topological sort during execution of the
//! different processes.

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

use log::debug;

use petgraph::graph::Graph;

use crate::config::ProcessConfig;
use crate::config::ProcessType;

/// Process information relevant for dependency resolution
///
/// Values found in `after_self` are process ids.
#[derive(Debug, PartialEq, Default)]
pub struct ProcessNode {
    after_self: Vec<usize>,

    predecessor_count: usize,

    finished: bool,
}

/// Errors occurring during dependency graph construction
#[derive(Debug, PartialEq)]
pub enum Error {
    /// The dependency graph contains a cycle involving the contained process id
    Cycle(usize),

    /// process id `.0` references an unknown program in its
    /// [`after`](crate::config::ProcessConfig::after) section at index `.1`.
    UnknownAfterReference(usize, usize),

    /// process id `.0` references an unknown program in its
    /// [`before`](crate::config::ProcessConfig::before) section at index `.1`.
    UnknownBeforeReference(usize, usize),

    /// process id `.0` references a cronjob in its
    /// [`after`](crate::config::ProcessConfig::after) section
    CronjobDependency(usize),
}

/// All data structures needed for dependency management.
#[derive(Debug, PartialEq)]
pub struct DependencyManager {
    nodes: HashMap<usize, ProcessNode>,

    runnable: VecDeque<usize>,

    runnable_archive: HashSet<usize>,
}

impl DependencyManager {
    /// Build a new dependency manager
    ///
    /// # Errors
    ///
    /// During construction the error conditions presented in [Error](Error) are
    /// checked.
    pub fn with_nodes(config: &[(usize, ProcessConfig)]) -> Result<Self, Error> {
        let name_dict = DependencyManager::build_name_dict(config);
        DependencyManager::validate_references(config, &name_dict)?;
        let nodes = DependencyManager::build_dependencies(config, &name_dict);
        let mut initial_runnables = DependencyManager::find_initial_runnables(&nodes);
        let result = DependencyManager {
            runnable: initial_runnables.clone(),
            nodes,
            runnable_archive: initial_runnables.drain(..).collect(),
        };

        result.check_for_cycles()?;
        Ok(result)
    }

    /// Check if any process can be run now
    pub fn has_runnables(&self) -> bool {
        !self.runnable.is_empty()
    }

    /// Get the next runnable process
    pub fn pop_runnable(&mut self) -> Option<usize> {
        self.runnable.pop_back()
    }

    /// Check if the given process id has ever reached runnable state according
    /// to [has_runnables](DependencyManager::has_runnables)
    pub fn is_runnable(&self, process_id: usize) -> bool {
        self.runnable_archive.contains(&process_id)
    }

    /// Remove the process id from the graph and compute new runnables from this
    /// event.
    pub fn notify_process_finished(&mut self, process_id: usize) {
        let process = self.nodes.get_mut(&process_id).expect("invalid process id");
        if process.finished {
            debug!(
                "Process {} has already triggered its dependants",
                process_id
            );
            return;
        }
        process.finished = true;
        for successor_index in self.nodes[&process_id].after_self.clone() {
            let mut successor = self.nodes.get_mut(&successor_index).expect("Invalid index");
            successor.predecessor_count -= 1;
            if successor.predecessor_count == 0 {
                // no need to remove `process` from successor's dependencies
                self.runnable.push_back(successor_index);
                self.runnable_archive.insert(successor_index);
            }
        }
    }

    /// Compute initially runnable processes without dependencies
    fn find_initial_runnables(nodes: &HashMap<usize, ProcessNode>) -> VecDeque<usize> {
        let mut result = VecDeque::new();
        nodes
            .iter()
            .filter(|(_, process)| process.predecessor_count == 0)
            .map(|(i, _)| result.push_back(*i))
            .for_each(drop);
        result
    }

    /// Build dependency relations between processes
    fn build_dependencies(
        config: &[(usize, ProcessConfig)],
        name_dict: &HashMap<String, usize>,
    ) -> HashMap<usize, ProcessNode> {
        let mut result = HashMap::with_capacity(config.len());

        for (k, _) in config {
            result.insert(*k, ProcessNode::default());
        }

        for (current_index, current_config) in config {
            let mut current = result
                .get_mut(current_index)
                .expect("Invalid index in name_dict");
            for successor_name in &current_config.before {
                let successor_index = name_dict
                    .get(successor_name)
                    .expect("Invalid index in name_dict");
                current.after_self.push(*successor_index);
            }

            current.predecessor_count += current_config.after.len();

            for successor_name in &current_config.before {
                let successor_index = name_dict
                    .get(successor_name)
                    .expect("Invalid index in name_dict");
                let mut successor = result
                    .get_mut(&successor_index)
                    .expect("Invalid index in name_dict");
                successor.predecessor_count += 1;
            }

            for predecessor_name in &current_config.after {
                let predecessor_index = name_dict
                    .get(predecessor_name)
                    .expect("Invalid index in name_dict");
                let predecessor = result
                    .get_mut(&predecessor_index)
                    .expect("Invalid index in name_dict");
                predecessor.after_self.push(*current_index);
            }
        }
        result
    }

    /// Check if the dependency graph allows for a topological order
    fn check_for_cycles(&self) -> Result<(), Error> {
        let mut graph = Graph::<_, _>::new();
        let mut node_dict = HashMap::new();

        for i in self.nodes.keys() {
            let node = graph.add_node(i);
            node_dict.insert(i, node);
        }

        for (i, node) in &self.nodes {
            for successor in &node.after_self {
                graph.add_edge(node_dict[&i], node_dict[&successor], 0);
            }
        }

        if let Err(cycle) = petgraph::algo::toposort(&graph, None) {
            let node_id = cycle.node_id();
            Err(Error::Cycle(**graph.node_weight(node_id).unwrap()))
        } else {
            Ok(())
        }
    }

    /// Index [ProcessConfigs](ProcessConfig) by [`name`](ProcessConfig::name)
    fn build_name_dict(descriptions: &[(usize, ProcessConfig)]) -> HashMap<String, usize> {
        let mut result = HashMap::new();

        for (i, desc) in descriptions {
            if result.contains_key(&desc.name) {
                panic!(
                    "Duplicate name {} should have already been eliminated",
                    desc.name
                );
            } else {
                result.insert(desc.name.to_owned(), *i);
            }
        }

        result
    }

    /// Check if name references to other processes are valid
    ///
    /// Validate references in [`before`](ProcessConfig::before) and
    /// [`after`](ProcessConfig::after) lists.
    ///
    /// # Errors
    ///
    /// The first found unknown reference reports an error
    fn validate_references(
        config: &[(usize, ProcessConfig)],
        name_dict: &HashMap<String, usize>,
    ) -> Result<(), Error> {
        for (prog_index, program) in config {
            for (after_index, dependency) in program.after.iter().enumerate() {
                match name_dict.get(dependency) {
                    None => {
                        return Err(Error::UnknownAfterReference(*prog_index, after_index));
                    }
                    Some(after_prog_index) => {
                        if let ProcessType::CronJob { .. } =
                            config[*after_prog_index].1.process_type
                        {
                            return Err(Error::CronjobDependency(*prog_index));
                        }
                    }
                }
            }
            for (before_index, dependency) in program.before.iter().enumerate() {
                if name_dict.get(dependency).is_none() {
                    return Err(Error::UnknownBeforeReference(*prog_index, before_index));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProcessConfig, ProcessType};

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
    #[should_panic]
    pub fn duplicate_name() {
        let config = vec![
            (0, ProcessConfig::new("first", vec![], vec![])),
            (1, ProcessConfig::new("first", vec![], vec![])),
        ];

        let _ = DependencyManager::with_nodes(&config);
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
                path: Some("".to_string()),
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
