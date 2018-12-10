use std::alloc::System;

#[global_allocator]
static A: System = System;

pub mod analyse;
pub mod cli_parser;
pub mod config;
pub mod logging;
pub mod runtime;
pub mod util;

use crate::config::config_parser;
use crate::runtime::process_manager::ProcessManager;

use log::info;

fn main() {
    let arguments = cli_parser::parse_arguments();
    logging::initialise(arguments.occurrences_of(cli_parser::FLAG_VERBOSE));

    info!("Starting up");

    let config_path = arguments
        .value_of(cli_parser::FLAG_CONFIG)
        .expect("Missing default value in cli_parser");
    info!("Config is at {}", config_path);

    info!("Parsing config");
    let process_tree = config_parser::parse_config(config_path);

    info!("Perform analysis on programs");
    let mut manager = ProcessManager::from(&process_tree);

    info!("Spawning processes");
    manager.start();
}
