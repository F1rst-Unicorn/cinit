//! Test program revealing information about its runtime
//!

extern crate capabilities;
extern crate clap;
extern crate nix;

use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::process::exit;
use std::thread;
use std::time;

use clap::App;
use clap::AppSettings;
use clap::Arg;

use nix::sys::signalfd;
use nix::unistd;

use capabilities::Capabilities;

fn main() {
    let arguments = App::new("cinit-harness")
        .setting(AppSettings::TrailingVarArg)
        .arg(
            Arg::with_name("return-code")
                .long("return-code")
                .short("r")
                .value_name("CODE")
                .help("The result code to return")
                .takes_value(true)
                .default_value("0"),
        )
        .arg(
            Arg::with_name("sleep")
                .long("sleep")
                .short("s")
                .value_name("SECONDS")
                .help("Sleep before termination")
                .takes_value(true)
                .default_value("0"),
        )
        .arg(
            Arg::with_name("output")
                .long("output")
                .short("o")
                .value_name("FILE")
                .help("Where to dump to")
                .takes_value(true)
                .default_value("test-output/harness.txt"),
        )
        .arg(
            Arg::with_name("rest")
                .help("Anything else to pass")
                .multiple(true),
        )
        .get_matches();

    dump(arguments.value_of("output").unwrap());

    let sleep_seconds = arguments
        .value_of("sleep")
        .unwrap()
        .parse::<u64>()
        .expect("invalid sleep seconds");

    let mask = signalfd::SigSet::all();
    let mut sfd = signalfd::SignalFd::with_flags(&mask, signalfd::SfdFlags::SFD_NONBLOCK)
        .expect("Could not setup signalfd");

    for _ in 0..sleep_seconds {
        thread::sleep(time::Duration::from_secs(1));
        if let Ok(Some(_)) = sfd.read_signal() {
            break;
        }
    }

    exit(
        arguments
            .value_of("return-code")
            .unwrap()
            .parse::<i32>()
            .expect("invalid return code"),
    );
}

fn dump(output: &str) {
    let mut file = File::create(output).unwrap_or_else(|_| panic!("Failed to open output file '{}'", output));

    file.write_fmt(format_args!("programs:\n"))
        .expect("Failed to open output file");

    file.write_fmt(format_args!("  - args:\n"))
        .expect("Failed to open output file");
    for arg in std::env::args() {
        file.write_fmt(format_args!("      - '{}'\n", arg.as_str()))
            .expect("Failed to dump");
    }

    file.write_fmt(format_args!(
        "    workdir: '{}'\n",
        std::env::current_dir()
            .expect("Could not get workdir")
            .to_str()
            .expect("Could not transform workdir str")
    ))
    .expect("Failed to dump");

    file.write_fmt(format_args!("    uid: {}\n", unistd::getuid()))
        .expect("Failed to dump");

    file.write_fmt(format_args!("    gid: {}\n", unistd::getgid()))
        .expect("Failed to dump");

    file.write_fmt(format_args!(
        "    pty: {}\n",
        unistd::isatty(std::io::stdout().as_raw_fd()).unwrap_or(false)
            && unistd::isatty(std::io::stderr().as_raw_fd()).unwrap_or(false)
    ))
    .expect("Failed to dump");

    file.write_fmt(format_args!("    capabilities:"))
        .expect("Failed to dump");
    let mut cap_string = Capabilities::from_current_proc()
        .expect("Could not get capabilities")
        .to_string();
    if cap_string.len() < 2 {
        file.write_fmt(format_args!(" []\n"))
            .expect("Failed to dump");
    } else {
        cap_string = cap_string.split_off(2);
        cap_string = cap_string
            .split('+')
            .next()
            .expect("Could not parse caps")
            .to_string();
        file.write_fmt(format_args!("\n")).expect("Failed to dump");
        for cap in cap_string.split(',') {
            file.write_fmt(format_args!("      - '{}'\n", cap.to_ascii_uppercase()))
                .expect("Failed to dump");
        }
    }

    file.write_fmt(format_args!("    env:\n"))
        .expect("Failed to open output file");
    for (key, value) in std::env::vars() {
        file.write_fmt(format_args!("      {}: '{}'\n", key, value))
            .expect("Failed to dump");
    }

    file.write_fmt(format_args!("\n")).expect("Failed to dump");
}
