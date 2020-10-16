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

//! Test program revealing information about its runtime
//!

use std::fs::File;
use std::io::Write;
use std::net::Shutdown;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixDatagram;
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
            Arg::with_name("status")
                .long("status")
                .short("S")
                .value_name("TEXT")
                .help("Status to report")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ready")
                .long("ready")
                .short("n")
                .help("Notify cinit that we are ready"),
        )
        .arg(
            Arg::with_name("rest")
                .help("Anything else to pass")
                .multiple(true),
        )
        .get_matches();

    if arguments.is_present("ready") {
        let sock = UnixDatagram::unbound().expect("failed to create socket");
        sock.connect("/run/cinit-notify.socket")
            .expect("failed to connect to notify socket");
        sock.send(b"READY=1").expect("failed to notify cinit");
        sock.shutdown(Shutdown::Both)
            .expect("failed to close socket");
    }

    if let Some(status) = arguments.value_of("status") {
        let sock = UnixDatagram::unbound().expect("failed to create socket");
        sock.connect("/run/cinit-notify.socket")
            .expect("failed to connect to notify socket");
        sock.send(("STATUS=".to_string() + status).as_bytes())
            .expect("failed to notify cinit");
        sock.shutdown(Shutdown::Both)
            .expect("failed to close socket");
    }

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
    let mut file =
        File::create(output).unwrap_or_else(|_| panic!("Failed to open output file '{}'", output));

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
        cap_string = cap_string
            .split('=')
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
