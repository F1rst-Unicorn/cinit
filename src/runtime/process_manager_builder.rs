use super::process_manager::ProcessManager;

use std::collections::HashMap;
use std::collections::VecDeque;
use std::process::exit;

use config::config::Config;
use runtime::process::{Process, ProcessDescription, ProcessNode, ProcessState};

use nix::sys::signalfd;

impl ProcessManager {
    pub fn from(config: Config) -> ProcessManager {
        let descriptions = ProcessManager::copy_processes(&config);

        let name_dict = ProcessManager::build_name_dict(&descriptions);

        let nodes = ProcessManager::build_dependencies(&config, &name_dict);

        let mut processes = ProcessManager::merge(descriptions, nodes);

        let runnable = ProcessManager::find_runnables(&mut processes);

        if runnable.len() == 0 {
            error!("No runnable processes found, check for cycles");
            trace!("No runnable processes found, check for cycles");
            exit(2);
        }

        ProcessManager {
            processes,
            name_dict,
            fd_dict: HashMap::new(),
            pid_dict: HashMap::new(),
            keep_running: true,
            runnable,
            epoll_file: -1,
            signal_fd: signalfd::SignalFd::new(&signalfd::SigSet::empty()).unwrap(),
        }
    }

    fn copy_processes(config: &Config) -> Vec<ProcessDescription> {
        let mut result = Vec::with_capacity(config.programs.len());
        for program in &config.programs {
            result.push(ProcessDescription::from(program));
        }
        result
    }

    fn build_name_dict(descriptions: &Vec<ProcessDescription>) -> HashMap<String, usize> {
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

    fn find_runnables(processes: &mut Vec<Process>) -> VecDeque<usize> {
        let mut result = VecDeque::new();
        for (i, process) in processes.iter_mut().enumerate() {
            if process.node_info.predecessor_count == 0 {
                result.push_back(i);
                process.description.state = ProcessState::Ready;
            }
        }
        result
    }

    fn merge(descriptions: Vec<ProcessDescription>, nodes: Vec<ProcessNode>) -> Vec<Process> {
        let mut result = Vec::with_capacity(descriptions.len());
        for (desc, node) in descriptions.into_iter().zip(nodes.into_iter()) {
            result.push(Process {
                description: desc,
                node_info: node,
            })
        }
        result
    }
}
