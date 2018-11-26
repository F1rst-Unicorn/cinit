use super::process_manager::ProcessManager;

use std::process::exit;

use config::config::Config;
use runtime::dependency_graph::{DependencyManager, Error};
use runtime::process::Process;
use runtime::process_map::ProcessMap;

use nix::sys::signalfd;

const EXIT_CODE: i32 = 2;

impl ProcessManager {
    pub fn from(config: Config) -> ProcessManager {
        let processes = config.programs.iter().map(Process::from).collect();

        let dependency_manager = DependencyManager::with_nodes(&config.programs);

        if let Err(err) = dependency_manager {
            match err {
                Error::Cycle(id) => {
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
                Error::Duplicate(id) => {
                    error!(
                        "Duplicate program found for name {}",
                        config.programs[id].name
                    );
                    trace!(
                        "Duplicate program found for name {}",
                        config.programs[id].name
                    );
                    exit(EXIT_CODE);
                }
            }
        }

        ProcessManager {
            process_map: ProcessMap::from(processes),
            keep_running: true,
            dependency_manager: dependency_manager.unwrap(),
            epoll_file: -1,
            signal_fd: signalfd::SignalFd::new(&signalfd::SigSet::empty()).unwrap(),
        }
    }
}
