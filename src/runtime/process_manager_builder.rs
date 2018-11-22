use super::process_manager::ProcessManager;

use std::collections::HashMap;
use std::process::exit;

use config::config::Config;
use runtime::dependency_graph::DependencyManager;
use runtime::process::Process;

use nix::sys::signalfd;

const EXIT_CODE: i32 = 2;

impl ProcessManager {
    pub fn from(config: Config) -> ProcessManager {
        let processes = config.programs.iter().map(Process::from).collect();

        let name_dict = ProcessManager::build_name_dict(&processes);

        let dependency_manager = DependencyManager::with_nodes(&config, &name_dict);

        if let Err(id) = dependency_manager {
            error!(
                "Found cycle involving process '{}'",
                config.programs[id].name
            );
            trace!(
                "Found cycle involving process '{}'",
                config.programs[id].name
            );
            exit(EXIT_CODE);
        }

        ProcessManager {
            processes,
            fd_dict: HashMap::new(),
            pid_dict: HashMap::new(),
            keep_running: true,
            dependency_manager: dependency_manager.unwrap(),
            epoll_file: -1,
            signal_fd: signalfd::SignalFd::new(&signalfd::SigSet::empty()).unwrap(),
        }
    }

    fn build_name_dict(descriptions: &Vec<Process>) -> HashMap<String, usize> {
        let mut result = HashMap::new();

        for (i, desc) in descriptions.into_iter().enumerate() {
            if result.contains_key(&desc.name) {
                error!("Duplicate program found for name {}", &desc.name);
                exit(EXIT_CODE);
            } else {
                result.insert(desc.name.to_owned(), i);
            }
        }

        result
    }
}
