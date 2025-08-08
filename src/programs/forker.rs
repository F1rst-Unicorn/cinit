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

//! Test program for SIGCHLD and fork behaviour

use std::process::exit;
use std::thread;
use std::time;

use nix::sys::signal::sigaction;
use nix::sys::signal::SaFlags;
use nix::sys::signal::SigAction;
use nix::sys::signal::SigHandler;
use nix::sys::signal::SigSet;
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::unistd::fork;
use nix::unistd::Pid;

static mut LOCK: i32 = 0;

extern "C" fn signal_handler(_: libc::c_int) {
    while waitpid(Pid::from_raw(-1), None).is_ok() {}
    unsafe {
        LOCK = 1;
    }
}

fn main() {
    let mut flags = SaFlags::empty();
    flags.insert(SaFlags::SA_RESTART);
    flags.insert(SaFlags::SA_NOCLDSTOP);
    unsafe {
        let res = sigaction(
            Signal::SIGCHLD,
            &SigAction::new(SigHandler::Handler(signal_handler), flags, SigSet::empty()),
        );
        if res.is_err() {
            println!("Could not setup signal handler: {:#?}", res.err());
            exit(-1);
        }
    }

    let fork_result = unsafe {
        // We are in a single-threaded program, so this unsafe call is ok
        // https://docs.rs/nix/0.19.0/nix/unistd/fn.fork.html#safety
        fork()
    };

    match fork_result {
        Ok(nix::unistd::ForkResult::Parent { .. }) => {
            let time = time::Duration::from_secs(2);
            thread::yield_now();
            thread::sleep(time);

            unsafe {
                if LOCK == 0 {
                    println!("Timeout waiting for child");
                    exit(-1);
                }
            }
        }
        Ok(nix::unistd::ForkResult::Child) => println!("Grandchild exitted"),
        _ => {
            println!("Forking failed!");
            exit(-1)
        }
    };
}
