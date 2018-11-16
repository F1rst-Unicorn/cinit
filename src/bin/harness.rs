extern crate clap;
extern crate nix;

use std::process::exit;

use clap::Arg;
use clap::App;
use clap::AppSettings;

use nix::unistd;


fn main() {

    let arguments = App::new("cinit-harness")
        .setting(AppSettings::TrailingVarArg)
        .arg(Arg::with_name("return-code")
            .long("return-code")
            .short("r")
            .value_name("CODE")
            .help("The result code to return")
            .takes_value(true)
            .default_value("0"))
        .arg(Arg::with_name("rest")
            .help("Anything else to pass")).get_matches();

    dump();

    exit(arguments.value_of("return-code").unwrap().parse::<i32>().expect("invalid return code"));
}

fn dump() {
    for arg in std::env::args() {
        print!("{} ", arg.as_str());
    }
    println!();

    println!("uid {}", unistd::getuid());
    println!("gid {}", unistd::getgid());

    for (key, value) in std::env::vars() {
        println!("{} = {}", key, value);
    }
}