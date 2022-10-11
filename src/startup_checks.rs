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

//! Check if cinit can run on this platform

use log::{debug, error, warn};
use nix::sys::utsname::uname;
use nix::unistd::getuid;

/// Unique exit code for this module
const EXIT_CODE: i32 = 5;

/// Terminate if requirements are not met
pub fn do_startup_checks() -> Result<(), i32> {
    check_kernel_version()?;
    check_user()
}

/// Terminate if linux version doesn't support capabilities
fn check_kernel_version() -> Result<(), i32> {
    let kernel_info = match uname() {
        Err(e) => {
            warn!("Could not read kernel version: {}", e);
            return Ok(());
        }
        Ok(v) => v,
    };
    let release_string = kernel_info.release().to_string_lossy();
    let mut release = release_string.split('.');
    if let Some(major_raw) = release.next() {
        if let Some(minor_raw) = release.next() {
            let major = major_raw.parse::<u32>();
            let minor = minor_raw.parse::<u32>();

            if major.is_err() || minor.is_err() {
                warn!(
                    "Could not determine kernel version from input '{}'",
                    release_string
                );
                return Ok(());
            }

            let major = major.unwrap();

            if major < 4 || (major == 4 && minor.unwrap() < 3) {
                error!("Your kernel is older than 4.3. Ambient capabilities");
                error!("are not supported on your kernel but are needed for");
                error!("cinit to work properly. Aborting");
                Err(EXIT_CODE)
            } else {
                debug!("Running on kernel version {}", release_string);
                Ok(())
            }
        } else {
            warn!(
                "Could not determine kernel version from input '{}'",
                release_string
            );
            Ok(())
        }
    } else {
        warn!(
            "Could not determine kernel version from input '{}'",
            release_string
        );
        Ok(())
    }
}

/// Terminate if not run as root
fn check_user() -> Result<(), i32> {
    let uid = getuid();
    if !uid.is_root() {
        error!("cinit is not running as root. This is");
        error!("needed to switch users and capabilities");
        error!("Aborting");
        Err(EXIT_CODE)
    } else {
        Ok(())
    }
}
