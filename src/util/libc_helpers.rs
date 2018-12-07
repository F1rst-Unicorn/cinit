//! Unsafe glue code for interaction with libc.
//!
//! NONE OF THE FUNCTIONS IS THREAD-SAFE!

use std::ffi::CString;
use std::os::unix::io::RawFd;
use std::ptr::null;

use nix::errno;
use nix::pty;

use std::io::Error;

use libc;

ioctl_read_bad!(get_terminal_size, libc::TIOCGWINSZ, pty::Winsize);

pub fn ttyname(fd: RawFd) -> Result<String, nix::Error> {
    unsafe {
        let raw_name = libc::ttyname(fd);
        if raw_name as (*const libc::c_char) == null() {
            Err(nix::Error::Sys(errno::Errno::from_i32(errno::errno())))
        } else {
            Ok(CString::from_raw(raw_name).to_str().unwrap().to_string().clone())
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
    unsafe {
        let null: *const libc::passwd = null();
        let cstring = CString::new(name).expect("Could not parse uid for libc");
        let raw_id: *const libc::passwd = libc::getpwnam(cstring.as_ptr() as *const i8);
        if raw_id == null {
            Err(nix::Error::last())
        } else {
            Ok((*raw_id).pw_uid)
        }
    }
}

pub fn is_uid_valid(uid: libc::uid_t) -> bool {
    unsafe {
        let null: *const libc::passwd = null();
        let raw_id: *const libc::passwd = libc::getpwuid(uid);
        raw_id != null
    }
}

pub fn group_to_gid(name: &str) -> Result<libc::gid_t, nix::Error> {
    unsafe {
        let null: *const libc::group = null();
        let cstring = CString::new(name).expect("Could not parse gid for libc");
        let raw_id: *const libc::group = libc::getgrnam(cstring.as_ptr() as *const i8);
        if raw_id == null {
            Err(nix::Error::last())
        } else {
            Ok((*raw_id).gr_gid)
        }
    }
}

pub fn is_gid_valid(gid: libc::gid_t) -> bool {
    unsafe {
        let null: *const libc::group = null();
        let raw_id: *const libc::group = libc::getgrgid(gid);
        raw_id != null
    }
}

pub fn map_to_errno(error: Error) -> nix::Error {
    match error.raw_os_error() {
        Some(errno) => nix::Error::Sys(nix::errno::Errno::from_i32(errno)),
        _ => nix::Error::Sys(nix::errno::Errno::UnknownErrno),
    }
}
