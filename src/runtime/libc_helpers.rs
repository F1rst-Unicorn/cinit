use std::ptr::null;
use std::ffi::CString;
use std::os::unix::io::RawFd;

use nix::pty;
use nix::errno;

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