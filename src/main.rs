use std::alloc::System;

#[global_allocator]
static A: System = System;

extern crate clap;
#[macro_use]
extern crate log;
extern crate log4rs;
#[macro_use]
extern crate serde_derive;
extern crate libc;
#[macro_use]
extern crate nix;
extern crate capabilities;
extern crate petgraph;
extern crate serde_yaml;
extern crate tera;

pub mod cli_parser;
pub mod config;
pub mod logging;
pub mod analyse;
pub mod runtime;
pub mod util;

use config::config_parser;
use runtime::process_manager::ProcessManager;

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
    let mut manager = ProcessManager::from(process_tree);

    info!("Spawning processes");
    manager.start();
}
