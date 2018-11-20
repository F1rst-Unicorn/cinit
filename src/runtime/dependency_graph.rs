use std::collections::VecDeque;


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

    pub fn with_nodes(nodes: Vec<ProcessNode>) -> Self {
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
}