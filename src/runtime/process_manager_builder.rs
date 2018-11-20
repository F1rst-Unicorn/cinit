use super::process_manager::ProcessManager;

use std::collections::HashMap;
use std::process::exit;

use config::config::Config;
use runtime::process::{Process};
use runtime::dependency_graph::DependencyManager;

use nix::sys::signalfd;

impl ProcessManager {
    pub fn from(config: Config) -> ProcessManager {
        let processes = config.programs.iter()
            .map(Process::from)
            .collect();

        let name_dict = ProcessManager::build_name_dict(&processes);

        let dependency_manager = DependencyManager::with_nodes(&config, &name_dict);

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
}
