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

use std::ffi::{CStr, CString};
use std::io::Error;
use std::os::unix::io::RawFd;
use std::ptr;

use nix::errno;
use nix::ioctl_read_bad;
use nix::pty;

use libc;

const BUFFER_LENGTH: usize = 4096;

ioctl_read_bad! {
    /// See `man 2 ioctl_tty` for general information about this call.
    ///
    /// # Safety
    ///
    /// Must be called with a valid file descriptor and a `&mut pty::Winsize`.
    get_terminal_size, libc::TIOCGWINSZ, pty::Winsize
}

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

pub fn prctl_one(option: libc::c_int, arg1: libc::c_ulong) -> Result<(), nix::Error> {
    unsafe {
        match libc::prctl(option, arg1) {
            -1 => Err(nix::Error::last()),
            _ => Ok(()),
        }
    }
}

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

pub fn user_to_uid(name: &str) -> Result<libc::uid_t, nix::Error> {
    let raw_name = CString::new(name).expect("Could not parse username for libc");
    let mut user_description: libc::passwd = unsafe { std::mem::zeroed() };
    let mut string_container = [0 as libc::c_char; BUFFER_LENGTH];
    let mut result = ptr::null_mut();

    unsafe {
        let ret = libc::getpwnam_r(
            raw_name.as_ptr(),
            &mut user_description,
            string_container.as_mut_ptr(),
            BUFFER_LENGTH,
            &mut result,
        );
        match ret {
            0 => {
                if result == &mut user_description {
                    Ok((*result).pw_uid)
                } else {
                    Err(nix::Error::last())
                }
            }
            _ => Err(nix::Error::last()),
        }
    }
}

pub fn uid_to_user(uid: libc::uid_t) -> Result<String, nix::Error> {
    unsafe {
        let user_description = uid_to_passwd_struct(uid)?;
        Ok(rescue_from_libc(user_description.0.pw_name))
    }
}

pub fn uid_to_homedir(uid: libc::uid_t) -> Result<String, nix::Error> {
    unsafe {
        let user_description = uid_to_passwd_struct(uid)?;
        Ok(rescue_from_libc(user_description.0.pw_dir))
    }
}

unsafe fn uid_to_passwd_struct(
    uid: libc::uid_t,
) -> Result<(libc::passwd, [libc::c_char; BUFFER_LENGTH]), nix::Error> {
    let mut user_description: libc::passwd = std::mem::zeroed();
    let mut string_container = [0 as libc::c_char; BUFFER_LENGTH];
    let mut result = ptr::null_mut();

    let ret = libc::getpwuid_r(
        uid,
        &mut user_description,
        string_container.as_mut_ptr(),
        BUFFER_LENGTH,
        &mut result,
    );
    match ret {
        0 => {
            if result == &mut user_description {
                Ok((user_description, string_container))
            } else {
                Err(nix::Error::last())
            }
        }
        _ => Err(nix::Error::last()),
    }
}

pub fn is_uid_valid(uid: libc::uid_t) -> bool {
    uid_to_user(uid).is_ok()
}

pub fn group_to_gid(name: &str) -> Result<libc::gid_t, nix::Error> {
    let raw_name = CString::new(name).expect("Could not parse groupname for libc");
    let mut group_description: libc::group = unsafe { std::mem::zeroed() };
    let mut string_container = [0 as libc::c_char; BUFFER_LENGTH];
    let mut result = ptr::null_mut();

    unsafe {
        let ret = libc::getgrnam_r(
            raw_name.as_ptr(),
            &mut group_description,
            string_container.as_mut_ptr(),
            BUFFER_LENGTH,
            &mut result,
        );
        match ret {
            0 => {
                if result == &mut group_description {
                    Ok((*result).gr_gid)
                } else {
                    Err(nix::Error::last())
                }
            }
            _ => Err(nix::Error::last()),
        }
    }
}

pub fn gid_to_group(gid: libc::gid_t) -> Result<String, nix::Error> {
    let mut group_description: libc::group = unsafe { std::mem::zeroed() };
    let mut string_container = [0 as libc::c_char; BUFFER_LENGTH];
    let mut result = ptr::null_mut();

    unsafe {
        let ret = libc::getgrgid_r(
            gid,
            &mut group_description,
            string_container.as_mut_ptr(),
            BUFFER_LENGTH,
            &mut result,
        );
        match ret {
            0 => {
                if result == &mut group_description {
                    Ok(rescue_from_libc((*result).gr_name))
                } else {
                    Err(nix::Error::last())
                }
            }
            _ => Err(nix::Error::last()),
        }
    }
}

pub fn is_gid_valid(gid: libc::gid_t) -> bool {
    gid_to_group(gid).is_ok()
}

pub fn map_to_errno(error: Error) -> nix::Error {
    let raw_error = error.raw_os_error();
    std::mem::drop(error);
    match raw_error {
        Some(errno) => nix::Error::Sys(nix::errno::Errno::from_i32(errno)),
        _ => nix::Error::Sys(nix::errno::Errno::UnknownErrno),
    }
}

/// Take a string returned from libc and copy it into a rust string.
/// This function makes sure subsequent calls to libc functions don't override
/// the string, leading to race conditions.
unsafe fn rescue_from_libc(string: *mut libc::c_char) -> String {
    CStr::from_ptr(string)
        .to_str()
        .expect("Could not rescue cstring")
        .to_string()
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_is_mapped_correctly() {
        assert_eq!(Ok(1409), user_to_uid("testuser"));
    }

    #[test]
    fn root_is_mapped_correctly() {
        assert_eq!(Ok(0), user_to_uid("root"));
    }

    #[test]
    fn invalid_user_yields_error() {
        assert!(user_to_uid("unknownuser").is_err());
    }

    #[test]
    fn testuser_is_found() {
        assert_eq!(Ok("testuser".to_string()), uid_to_user(1409));
    }

    #[test]
    fn root_is_found() {
        assert_eq!(Ok("root".to_string()), uid_to_user(0));
    }

    #[test]
    fn invalid_user_is_not_found() {
        assert!(uid_to_user(1410).is_err());
    }

    #[test]
    fn testuser_homedir_is_correct() {
        assert_eq!(Ok("/home/testuser".to_string()), uid_to_homedir(1409));
    }

    #[test]
    fn invalid_user_homedir_is_reported() {
        assert!(uid_to_homedir(1410).is_err());
    }

    #[test]
    fn root_is_valid() {
        assert!(is_uid_valid(0));
    }

    #[test]
    fn testuser_is_valid() {
        assert!(is_uid_valid(1409));
    }

    #[test]
    fn invalid_user() {
        assert!(!is_uid_valid(1410));
    }

    #[test]
    fn testgroup_is_found() {
        assert_eq!(Ok("testgroup".to_string()), gid_to_group(1409));
    }

    #[test]
    fn invalid_group_is_not_found() {
        assert!(gid_to_group(1410).is_err());
    }

    #[test]
    fn root_group_is_valid() {
        assert!(is_gid_valid(0));
    }

    #[test]
    fn testgroup_is_valid() {
        assert!(is_gid_valid(1409));
    }

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
