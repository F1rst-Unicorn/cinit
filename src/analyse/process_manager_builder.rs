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

use std::process::exit;

use crate::config::{Config, ProcessConfig, ProcessType};
use crate::runtime::cronjob::{Cron, Error as CronError};
use crate::runtime::dependency_graph::{DependencyManager, Error};
use crate::runtime::process::Process;
use crate::runtime::process_manager::ProcessManager;
use crate::runtime::process_map::ProcessMap;

use nix::sys::signalfd;

use log::{error, trace};

const EXIT_CODE: i32 = 2;

impl ProcessManager {
    pub fn from(config: &Config) -> ProcessManager {
        let mut processes = Vec::new();
        for program_config in &config.programs {
            let program = Process::from(program_config);

            if let Err(error) = program {
                error!("Program {} contains error: {}", program_config.name, error);
                trace!("Program {} contains error: {}", program_config.name, error);
                exit(EXIT_CODE);
            } else {
                processes.push(program.unwrap());
            }
        }

        let dependency_manager = build_dependency_manager(&config);
        let cron = build_cron(&config);

        ProcessManager {
            process_map: ProcessMap::from(processes),
            keep_running: true,
            dependency_manager,
            cron,
            epoll_fd: -1,
            status_fd: -1,
            signal_fd: signalfd::SignalFd::new(&signalfd::SigSet::empty())
                .expect("Could not create signalfd"),
        }
    }
}

fn build_dependency_manager(config: &Config) -> DependencyManager {
    let input: Vec<(usize, ProcessConfig)> = config
        .programs
        .iter()
        .map(Clone::clone)
        .enumerate()
        .filter(|(_, p)| p.process_type == ProcessType::Oneshot)
        .collect();

    let dependency_manager = DependencyManager::with_nodes(&input);

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
            }
            Error::UnknownAfterReference(prog_index, after_index) => {
                error!(
                    "Unknown 'after' dependency '{}' of program {}",
                    config.programs[prog_index].after[after_index],
                    config.programs[prog_index].name
                );
                trace!(
                    "Unknown 'after' dependency '{}' of program {}",
                    config.programs[prog_index].after[after_index],
                    config.programs[prog_index].name
                );
            }
            Error::UnknownBeforeReference(prog_index, before_index) => {
                error!(
                    "Unknown 'before' dependency '{}' of program {}",
                    config.programs[prog_index].before[before_index],
                    config.programs[prog_index].name
                );
                trace!(
                    "Unknown 'before' dependency '{}' of program {}",
                    config.programs[prog_index].before[before_index],
                    config.programs[prog_index].name
                );
            }
        }
        exit(EXIT_CODE);
    } else {
        dependency_manager.unwrap()
    }
}

fn build_cron(config: &Config) -> Cron {
    let input: Vec<(usize, ProcessConfig)> = config
        .programs
        .iter()
        .map(Clone::clone)
        .enumerate()
        .filter(|(_, p)| {
            if let ProcessType::CronJob { .. } = p.process_type {
                true
            } else {
                false
            }
        })
        .collect();

    let cron = Cron::with_jobs(&input);

    if let Err(error) = cron {
        match error {
            CronError::TimeParseError(message, id) => {
                error!(
                    "Timer parse error for program '{}': {}",
                    config.programs[id].name, message
                );
                trace!(
                    "Timer parse error for program '{}': {}",
                    config.programs[id].name,
                    message
                );
            }
        }
        exit(EXIT_CODE);
    } else {
        cron.unwrap()
    }
}
