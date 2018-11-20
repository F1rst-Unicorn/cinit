use std::collections::VecDeque;
use std::collections::HashMap;

use config::config::Config;


/// Process information relevant for dependency resolution
/// via ongoing topological sorting
#[derive(Debug)]
pub struct ProcessNode {
    pub before: Vec<usize>,

    pub predecessor_count: usize,
}

#[derive(Debug)]
pub struct DependencyManager {
    nodes: Vec<ProcessNode>,

    pub runnable: VecDeque<usize>,
}

impl DependencyManager {

    pub fn with_nodes(config: &Config, name_dict: &HashMap<String, usize>) -> Self {
        let nodes = DependencyManager::build_dependencies(config, name_dict);
        DependencyManager {
            runnable: DependencyManager::find_initial_runnables(&nodes),
            nodes,
        }
    }

    pub fn has_runnables(&self) -> bool {
        ! self.runnable.is_empty()
    }

    pub fn pop_runnable(&mut self) -> Option<usize> {
        self.runnable.pop_back()
    }

    pub fn notify_process_finished(&mut self, process: usize) -> Vec<usize> {
        let mut result = Vec::new();
        for successor_index in self.nodes[process].before.clone() {
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
        for (i, process) in nodes.iter().enumerate() {
            if process.predecessor_count == 0 {
                result.push_back(i);
            }
        }
        result
    }

    fn build_dependencies(config: &Config, name_dict: &HashMap<String, usize>) -> Vec<ProcessNode> {
        let mut result = Vec::with_capacity(config.programs.len());

        for _ in 0..config.programs.len() {
            result.push(ProcessNode {
                before: Vec::new(),
                predecessor_count: 0,
            });
        }

        for process_config in &config.programs {
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
                    current.before.push(predecessor_index);
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
                dependency.before.push(current_index);
            }
        }

        result
    }
}