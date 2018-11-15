extern crate clap;
#[macro_use]
extern crate log;
extern crate simple_logger;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

mod cli_parser;
mod config_parser;
mod process_tree;

use log::Level;

fn main() {

    let arguments = cli_parser::parse_arguments();
    initialise_log(&arguments);

    info!("Starting up");

    let config_path = arguments.value_of("config").unwrap();
    info!("Config is at {}", config_path );

    let process_tree = config_parser::parse_config(config_path);
    debug!("Config dump: {:?}", process_tree);
    info!("Config parsed, starting children");

    process_tree.start();
}

fn initialise_log(arguments: &clap::ArgMatches) {
    if arguments.is_present("verbose") {
        simple_logger::init_with_level(Level::Trace).unwrap();
    } else {
        simple_logger::init_with_level(Level::Info).unwrap();
    }
}
