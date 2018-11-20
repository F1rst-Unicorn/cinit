use super::process_manager::ProcessManager;

use std::collections::HashMap;
use std::process::exit;

use config::config::Config;
use runtime::process::{Process};
use runtime::dependency_graph::{DependencyManager, ProcessNode};

use nix::sys::signalfd;

impl ProcessManager {
    pub fn from(config: Config) -> ProcessManager {
        let processes = config.programs.iter()
            .map(Process::from)
            .collect();

        let name_dict = ProcessManager::build_name_dict(&processes);

        let nodes = ProcessManager::build_dependencies(&config, &name_dict);

        let dependency_manager = DependencyManager::with_nodes(nodes);

        ProcessManager {
            processes,
            fd_dict: HashMap::new(),
            pid_dict: HashMap::new(),
            keep_running: true,
            dependency_manager,
            epoll_file: -1,
            signal_fd: signalfd::SignalFd::new(&signalfd::SigSet::empty()).unwrap(),
        }
    }

    fn build_name_dict(descriptions: &Vec<Process>) -> HashMap<String, usize> {
        let mut result = HashMap::new();

        for (i, desc) in descriptions.into_iter().enumerate() {
            if result.contains_key(&desc.name) {
                error!("Duplicate program found for name {}", &desc.name);
                exit(2);
            } else {
                result.insert(desc.name.to_owned(), i);
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
