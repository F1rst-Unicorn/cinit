use clap::Arg;
use clap::App;

pub fn parse_arguments<'a>() -> clap::ArgMatches<'a> {

    let app = App::new("cinit")
            .version(env!("CARGO_PKG_VERSION"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .arg(Arg::with_name("config")
                .long("config")
                .value_name("PATH")
                .help("The config file or directory to run with")
                .takes_value(true)
                .default_value("/etc/cinit.yml"))
            .arg(Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Output information while running")
                .takes_value(false));
    app.get_matches()
}
