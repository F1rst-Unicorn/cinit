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

//! Parse command line

use clap::App;
use clap::Arg;

/// Control cinit's logging verbosity
pub const FLAG_VERBOSE: &str = "verbose";

/// Control cinit's configuration root
pub const FLAG_CONFIG: &str = "config";

/// Transform command line into [clap](clap) struct
pub fn parse_arguments<'a>() -> clap::ArgMatches<'a> {
    let app = App::new("cinit")
        .version(concat!(
            env!("CARGO_PKG_VERSION"),
            " ",
            env!("VERGEN_SHA"),
            " ",
            env!("VERGEN_BUILD_TIMESTAMP")
        ))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("config")
                .short("c")
                .long(FLAG_CONFIG)
                .value_name("PATH")
                .help("The config file or directory to run with")
                .takes_value(true)
                .default_value("/etc/cinit.yml"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long(FLAG_VERBOSE)
                .help("Output information while running")
                .multiple(true)
                .takes_value(false),
        );
    app.get_matches()
}
