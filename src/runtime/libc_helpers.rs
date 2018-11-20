use std::ptr::null;
use std::ffi::CString;
use std::os::unix::io::RawFd;

use nix::pty;
use nix::errno;

use std::io::Error;

use libc;

ioctl_read_bad!(get_terminal_size, libc::TIOCGWINSZ, pty::Winsize);

pub fn ttyname(fd: RawFd) -> Result<CString, nix::Error> {
    unsafe {
        let raw_name = libc::ttyname(fd);
        if raw_name as (*const libc::c_char) == null() {
            Err(nix::Error::Sys(errno::Errno::from_i32(errno::errno())))
        } else {
            Ok(CString::from_raw(raw_name))
        }
    }
}

pub fn prctl_one(option: libc::c_int, arg1: libc::c_ulong) -> Result<(), nix::Error> {
    unsafe {
        match libc::prctl(option, arg1) {
            -1 => {
                Err(nix::Error::last())
            }
            _ => {
                Ok(())
            }
        }
    }
}

pub fn prctl_four(option: libc::c_int,
                  arg1: libc::c_ulong,
                  arg2: libc::c_ulong,
                  arg3: libc::c_ulong,
                  arg4: libc::c_ulong) -> Result<(), nix::Error> {
    unsafe {
        match libc::prctl(option, arg1, arg2, arg3, arg4) {
            -1 => {
                Err(nix::Error::last())
            }
            _ => {
                Ok(())
            }
        }
    }
}

pub fn map_to_errno(error: Error) -> nix::Error {
    match error.raw_os_error() {
        Some(errno) => {
            nix::Error::Sys(nix::errno::Errno::from_i32(errno))
        }
        _ => {
            nix::Error::Sys(nix::errno::Errno::UnknownErrno)
        }
    }
}