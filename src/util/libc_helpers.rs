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

//! Unsafe glue code for interaction with libc.
//!
//! NONE OF THE FUNCTIONS IS THREAD-SAFE!

use std::ffi::CStr;
use std::io::Error;
use std::os::unix::io::RawFd;

use nix::errno;
use nix::ioctl_read_bad;
use nix::pty;

ioctl_read_bad! {
    /// See `man 2 ioctl_tty` for general information about this call.
    ///
    /// # Safety
    ///
    /// Must be called with a valid file descriptor and a `&mut pty::Winsize`.
    get_terminal_size, libc::TIOCGWINSZ, pty::Winsize
}

/// Safe wrapper around `ttyname()`, see `man 3 ttyname`.
pub fn ttyname(fd: RawFd) -> Result<String, nix::Error> {
    unsafe {
        let raw_name = libc::ttyname(fd);
        if raw_name.is_null() {
            Err(nix::Error::Sys(errno::Errno::from_i32(errno::errno())))
        } else {
            Ok(rescue_from_libc(raw_name))
        }
    }
}

/// Safe wrapper around `prctl()` with one argument, see `man 2 prctl`.
pub fn prctl_one(option: libc::c_int, arg1: libc::c_ulong) -> Result<(), nix::Error> {
    unsafe {
        match libc::prctl(option, arg1) {
            -1 => Err(nix::Error::last()),
            _ => Ok(()),
        }
    }
}

/// Safe wrapper around `prctl()` with four argument, see `man 2 prctl`.
pub fn prctl_four(
    option: libc::c_int,
    arg1: libc::c_ulong,
    arg2: libc::c_ulong,
    arg3: libc::c_ulong,
    arg4: libc::c_ulong,
) -> Result<(), nix::Error> {
    unsafe {
        match libc::prctl(option, arg1, arg2, arg3, arg4) {
            -1 => Err(nix::Error::last()),
            _ => Ok(()),
        }
    }
}

/// Transform error types by matching `errno`
pub fn map_to_errno(error: Error) -> nix::Error {
    let raw_error = error.raw_os_error();
    std::mem::drop(error);
    match raw_error {
        Some(errno) => nix::Error::Sys(nix::errno::Errno::from_i32(errno)),
        _ => nix::Error::Sys(nix::errno::Errno::UnknownErrno),
    }
}

/// Take a byte array representing a C String with unknown length. In particular
/// the length can be smaller than the slice, indicated by a zero byte in the
/// middle of the string
pub fn slice_to_string(buffer: &[u8]) -> String {
    unsafe { rescue_from_libc(buffer.as_ptr() as *const libc::c_char) }
}

/// Take a string returned from libc and copy it into a rust string.
/// This function makes sure subsequent calls to libc functions don't override
/// the string, leading to race conditions.
unsafe fn rescue_from_libc(string: *const libc::c_char) -> String {
    CStr::from_ptr(string)
        .to_str()
        .expect("Could not rescue cstring")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rescuing_works() {
        unsafe {
            let input = "teststring".as_ptr() as *mut libc::c_char;
            assert_ne!(input, rescue_from_libc(input).as_ptr() as *mut libc::c_char);
        }
    }

    #[test]
    fn rescuing_empty_works() {
        unsafe {
            let input = [0, 0, 61, 61, 61, 61, 61, 61].as_ptr() as *mut libc::c_char;
            assert_eq!("".to_string(), rescue_from_libc(input));
        }
    }
}
