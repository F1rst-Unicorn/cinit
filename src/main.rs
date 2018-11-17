extern crate clap;
#[macro_use]
extern crate log;
extern crate simple_logger;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate nix;

mod cli_parser;
mod config;
mod runtime;

use log::Level;
use config::config_parser;
use runtime::process_manager::ProcessManager;

fn main() {

    let arguments = cli_parser::parse_arguments();
    initialise_log(arguments.is_present(cli_parser::FLAG_VERBOSE));

    info!("Starting up");

    let config_path = arguments.value_of(cli_parser::FLAG_CONFIG)
            .expect("Missing default value in cli_parser");
    info!("Config is at {}", config_path );

    info!("Parsing config");
    let process_tree = config_parser::parse_config(config_path);

    info!("Perform analysis on programs");
    let mut manager = ProcessManager::from(process_tree);

    info!("Spawning processes");
    manager.start();
}

fn initialise_log(verbose: bool) {
    if verbose {
        simple_logger::init_with_level(Level::Trace).unwrap();
    } else {
        simple_logger::init_with_level(Level::Info).unwrap();
    }
}
