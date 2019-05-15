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

use log::{debug, error, warn};
use nix::sys::utsname::uname;
use nix::unistd::getuid;
use std::process::exit;

const EXIT_CODE: i32 = 5;

pub fn do_startup_checks() {
    check_kernel_version();
    check_user();
}

fn check_kernel_version() {
    let kernel_info = uname();
    let mut release = kernel_info.release().split('.');
    if let Some(major_raw) = release.next() {
        if let Some(minor_raw) = release.next() {
            let major = major_raw.parse::<u32>();
            let minor = minor_raw.parse::<u32>();

            if major.is_err() || minor.is_err() {
                warn!(
                    "Could not determine kernel version from input '{}'",
                    kernel_info.release()
                );
                return;
            }

            let major = major.unwrap();

            if major < 4 || (major == 4 && minor.unwrap() < 3) {
                error!("Your kernel is older than 4.3. Ambient capabilities");
                error!("are not supported on your kernel but are needed for");
                error!("cinit to work properly. Aborting");
                exit(EXIT_CODE);
            } else {
                debug!("Running on kernel version {}", kernel_info.release());
            }
        } else {
            warn!(
                "Could not determine kernel version from input '{}'",
                kernel_info.release()
            );
        }
    } else {
        warn!(
            "Could not determine kernel version from input '{}'",
            kernel_info.release()
        );
    }
}

fn check_user() {
    let uid = getuid();
    if !uid.is_root() {
        error!("cinit is not running as root. This is");
        error!("needed to switch users and capabilities");
        error!("Aborting");
        exit(EXIT_CODE);
    }
}
