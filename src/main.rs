//! Init program for UNIX processes. Original development was done
//! [here](https://github.com/vs-eth/scinit)
//!
//! ## Configuration
//!
//! cinit takes its configuration via yaml files. They can look like this:
//!
//! ```yml
//! programs:
//!   - name: Some descriptive name
//!
//!     # The path of the binary to run
//!     path: /usr/bin/echo
//!
//!     # A list of arguments to pass to the program
//!     args:
//!       - hello
//!       - world
//!
//!     # See Program Types
//!     type: oneshot
//!
//!     # If none is given, root is used
//!     uid: 0
//!     gid: 0
//!     user: root
//!     group: root
//!
//!     # Specify dependencies, see below
//!     before:
//!       - other program name
//!     after:
//!       - other program name
//!
//!     # Emulate a pseudo-terminal for this program
//!     pty: false
//!
//!     # Give capabilities to this program
//!     capabilities:
//!       - CAP_NET_RAW
//!
//!     # Pass environment variables
//!     env:
//!       PWD: /home/user
//!       # If no value is given, it is forwarded from cinit
//!       PASSWORD:
//! ```
//!
//! If a file is passed via command line it is the only file used. Passing a
//! directory makes cinit traverse it recursively and taking all found files as
//! configuration. If no path is given /etc/cinit.yml is used.
//!
//!
//! ## Usage
//!
//! ```text
//! cinit 0.1.0
//! init daemon for other programs, suitable for containers
//!
//! USAGE:
//!     cinit [FLAGS] [OPTIONS]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!     -v, --verbose    Output information while running
//!
//! OPTIONS:
//!         --config <PATH>    The config file or directory to run with [default: /etc/cinit.yml]
//! ```
//!
//! ## Program types
//!
//! Supported are `oneshot` and `service`. There is no difference as of now.
//!
//! ## Environment
//!
//! By default the following environment variables will be forwarded from the
//! cinit process to the programs:
//!
//! * `HOME`
//! * `LANG`
//! * `LANGUAGE`
//! * `LOGNAME`
//! * `PATH`
//! * `PWD`
//! * `SHELL`
//! * `TERM`
//! * `USER`
//!
//! Additional parameters may be specified. If no value is given, cinit will forward
//! the value from its own environment.
//!
//! ## Capabilities
//!
//! Processes can be restricted in what they are allowed to do. This can also
//! mean that non-root process get elevated capabilities. See
//! [here](http://man7.org/linux/man-pages/man7/capabilities.7.html)
//! for a list of all capabilities.
//!
//! ## Dependencies
//!
//! Programs are allowed to depend on each other via the `before` and `after`
//! fields. Dendendant processes will only be started once all their
//! dependencies have terminated. Refer to other programs in the config via
//! their `name` field.
//!
//! If the dependencies form a cycle, this is reported before any process is
//! started and cinit terminates.
//!

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
extern crate serde_yaml;
extern crate capabilities;
extern crate petgraph;

pub mod cli_parser;
pub mod config;
pub mod runtime;
pub mod logging;

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
