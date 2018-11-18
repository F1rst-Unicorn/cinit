use clap::App;
use clap::Arg;

pub const FLAG_VERBOSE: &str = "verbose";
pub const FLAG_CONFIG: &str = "config";

pub fn parse_arguments<'a>() -> clap::ArgMatches<'a> {
    let app = App::new("cinit")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("config")
                .long(FLAG_CONFIG)
                .value_name("PATH")
                .help("The config file or directory to run with")
                .takes_value(true)
                .default_value("/etc/cinit.yml"),
        ).arg(
            Arg::with_name("verbose")
                .short("v")
                .long(FLAG_VERBOSE)
                .help("Output information while running")
                .multiple(true)
                .takes_value(false),
        );
    app.get_matches()
}
