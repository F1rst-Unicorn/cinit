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

//! container init system
//!
//! This documentation targets developers of cinit. If you want to learn how to
//! use cinit, see [`doc/README.md`](https://gitlab.com/veenj/cinit/-/blob/master/doc/README.md).
//!
//! Run processes and take init (PID 0) responsibilities inside containers.
//!
//! # Features
//!
//! * **Clean Termination**: If cinit receives a termination signal it forwards
//!   it to all its child processes so they can terminate properly.
//!
//! * **Capabilities**: Set Linux Capabilities on child processes.
//!
//! * **Environment Sanitisation**: Control what environment variables are
//!   available in child processes using an expressive template language.
//!
//! * **Dependency Trees**: Declare a relative ordering on the processes
//!   being spawned.
//!
//! * **Cron jobs**: Run processes repeatedly using a cron-like API.
//!
//! * **Zombie Reaping**: Adopt and terminate abandoned child processes.
//!
//! # Architecture
//!
//! cinit execution is split into three phases:
//!
//! 1. [Configuration Collection](config):
//!    Read all configuration files, merge them into a single structure
//!    and perform basic validity checks.
//!
//! 2. [Analysis](analyse):
//!    Do validation analysis and build data structures for later execution
//!
//! 3. [Runtime](runtime):
//!    Start executing processes and manage runtime events.

use std::alloc::System;

#[global_allocator]
static A: System = System;

pub mod analyse;
pub mod cli_parser;
pub mod config;
pub mod logging;
pub mod runtime;
pub mod startup_checks;
pub mod util;

use crate::config::config_parser;
use crate::runtime::process_manager::ProcessManager;

use log::info;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let arguments = cli_parser::parse_arguments();
    logging::initialise(arguments.occurrences_of(cli_parser::FLAG_VERBOSE));

    info!("Starting up");
    if let Err(exit_code) = startup_checks::do_startup_checks() {
        return exit_code;
    }

    let config_path = arguments
        .value_of(cli_parser::FLAG_CONFIG)
        .expect("Missing default value in cli_parser");
    info!("Config is at {}", config_path);

    info!("Parsing config");
    let process_tree = match config_parser::parse_config(config_path) {
        Err(exit_code) => {
            return exit_code;
        }
        Ok(v) => v,
    };

    info!("Perform analysis on programs");
    let mut manager = match ProcessManager::from(&process_tree) {
        Err(exit_code) => {
            return exit_code;
        }
        Ok(v) => v,
    };

    info!("Spawning processes");
    manager.start()
}
