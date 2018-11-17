extern crate clap;
extern crate nix;

use std::process::exit;
use std::thread;
use std::time;
use std::fs::File;
use std::io::Write;

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
        .arg(Arg::with_name("sleep")
            .long("sleep")
            .short("s")
            .value_name("SECONDS")
            .help("Sleep before termination")
            .takes_value(true)
            .default_value("0"))
        .arg(Arg::with_name("output")
            .long("output")
            .short("o")
            .value_name("FILE")
            .help("Where to dump to")
            .takes_value(true)
            .default_value("test-output/harness.txt"))
        .arg(Arg::with_name("rest")
            .help("Anything else to pass")
            .multiple(true)).get_matches();

    dump(arguments.value_of("output").unwrap());

    let sleep_seconds = arguments.value_of("sleep").unwrap()
            .parse::<u64>().expect("invalid sleep seconds");
    thread::sleep(time::Duration::from_secs(sleep_seconds));

    exit(arguments.value_of("return-code").unwrap()
            .parse::<i32>().expect("invalid return code"));
}

fn dump(output: &str) {
    let mut file = File::create(output)
            .expect("Failed to open output file");

    for arg in std::env::args() {
        file.write_fmt(format_args!("{} ", arg.as_str()))
            .expect("Failed to dump");
    }
    file.write_fmt(format_args!("\n"))
        .expect("Failed to dump");


    file.write_fmt(format_args!("uid {}\n", unistd::getuid()))
        .expect("Failed to dump");

    file.write_fmt(format_args!("gid {}\n", unistd::getgid()))
        .expect("Failed to dump");

    for (key, value) in std::env::vars() {
        file.write_fmt(format_args!("{} = {}\n", key, value))
            .expect("Failed to dump");
    }

    file.write_fmt(format_args!("\n"))
        .expect("Failed to dump");
}