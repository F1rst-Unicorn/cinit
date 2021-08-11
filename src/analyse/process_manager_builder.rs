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

//! Build the [ProcessManager](crate::runtime::process_manager::ProcessManager)
//! for runtime execution.
//!
//! # Precomputations
//!
//! Transform each contained [ProcessConfig](ProcessConfig) into a
//! [Process](Process). Every process is assigned an arbitrary, unique process
//! id, not to be confused with UNIX PIDs which are only valid while the process
//! runs.
//!
//! Build the dependency graph for efficient unblocking at runtime.
//!
//! Parse cron expressions and set up their timers.
//!
//! # Validation
//!
//! A cycle in the dependency graph raises an error.
//!
//! Processes with unknown dependencies raise an error.
//!
//! Dependencies on cronjobs raise an error.
//!
//! Errors in the cronjob configurations are forwarded.

use crate::config::{Config, ProcessConfig, ProcessType};
use crate::runtime::cronjob::{Cron, Error as CronError};
use crate::runtime::dependency_graph::{DependencyManager, Error};
use crate::runtime::process::Process;
use crate::runtime::process_manager::ProcessManager;
use crate::runtime::process_map::ProcessMap;

use nix::sys::signalfd;

use log::{error, trace};

/// Unique exit code for this module
const EXIT_CODE: i32 = 2;

impl ProcessManager {
    /// See [analysis phase](crate::analyse::process_manager_builder)
    pub fn from(config: &Config) -> Result<ProcessManager, i32> {
        let mut processes = Vec::new();
        for program_config in &config.programs {
            let program = Process::from(program_config);

            if let Err(error) = program {
                error!("Program {} contains error: {}", program_config.name, error);
                trace!("Program {} contains error: {}", program_config.name, error);
                return Err(EXIT_CODE);
            } else {
                processes.push(program.unwrap());
            }
        }

        let dependency_manager = build_dependency_manager(config);
        let cron = build_cron(config);

        Ok(ProcessManager {
            process_map: ProcessMap::from(processes),
            keep_running: true,
            dependency_manager: dependency_manager?,
            cron: cron?,
            epoll_fd: -1,
            status_fd: -1,
            notify_fd: -1,
            signal_fd: signalfd::SignalFd::new(&signalfd::SigSet::empty())
                .expect("Could not create signalfd"),
            exit_code: 0,
        })
    }
}

/// Build the [DependencyManager](DependencyManager)
///
/// Every process is assigned an arbitrary unique id using the same procedure as
/// in [build_cron()](build_cron).
///
/// Errors during building are forwarded and terminate cinit.
fn build_dependency_manager(config: &Config) -> Result<DependencyManager, i32> {
    let input: Vec<(usize, ProcessConfig)> = config
        .programs
        .iter()
        .map(Clone::clone)
        .enumerate()
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
            Error::CronjobDependency(prog_index) => {
                error!(
                    "Program {} contains error: Depending on cronjobs is not allowed",
                    config.programs[prog_index].name
                );
                trace!(
                    "Program {} contains error: Depending on cronjobs is not allowed",
                    config.programs[prog_index].name
                );
            }
        }
        Err(EXIT_CODE)
    } else {
        Ok(dependency_manager.unwrap())
    }
}

/// Build the [Cron](Cron)
///
/// Every process is assigned an arbitrary unique id using the same procedure as
/// in [build_dependency_manager()](build_dependency_manager).
///
/// Errors during building are forwarded and terminate cinit.
fn build_cron(config: &Config) -> Result<Cron, i32> {
    let input: Vec<(usize, ProcessConfig)> = config
        .programs
        .iter()
        .map(Clone::clone)
        .enumerate()
        .filter(|(_, p)| matches!(p.process_type, ProcessType::CronJob { .. }))
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
        Err(EXIT_CODE)
    } else {
        Ok(cron.unwrap())
    }
}
