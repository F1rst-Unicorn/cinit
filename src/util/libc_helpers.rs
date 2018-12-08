//! Unsafe glue code for interaction with libc.
//!
//! NONE OF THE FUNCTIONS IS THREAD-SAFE!

use std::ffi::{CStr, CString};
use std::os::unix::io::RawFd;
use std::ptr::null;
use std::ptr;

use nix::errno;
use nix::pty;

use std::io::Error;

use libc;

const BUFFER_LENGTH: usize = 4096;

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
    let cstring = CString::new(name).expect("Could not parse username for libc");
    let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut cbuf = [0 as libc::c_char; BUFFER_LENGTH];
    let mut res = ptr::null_mut();

    unsafe {
        let ret = libc::getpwnam_r(cstring.as_ptr(), &mut pwd, cbuf.as_mut_ptr(), BUFFER_LENGTH, &mut res);
        match ret {
            0 => {
                if res == &mut pwd {
                    return Ok((*res).pw_uid);
                } else {
                    return Err(nix::Error::last());
                }
            },
            _ => {
                return Err(nix::Error::last());
            }
        }
    }
}

pub fn uid_to_user(uid: libc::uid_t) -> Result<String, nix::Error> {
    unsafe {
        let passwd = uid_to_passwd_struct(uid)?;
        return Ok(CStr::from_ptr(passwd.0.pw_name).to_string_lossy().into_owned());
    }
}

pub fn uid_to_homedir(uid: libc::uid_t) -> Result<String, nix::Error> {
    unsafe {
        let passwd = uid_to_passwd_struct(uid)?;
        return Ok(CStr::from_ptr(passwd.0.pw_dir).to_string_lossy().into_owned());
    }
}

unsafe fn uid_to_passwd_struct(uid: libc::uid_t) -> Result<(libc::passwd, [libc::c_char; BUFFER_LENGTH]), nix::Error> {
    let mut pwd: libc::passwd = std::mem::zeroed();
    let mut cbuf = [0 as libc::c_char; BUFFER_LENGTH];
    let mut res = ptr::null_mut();

    let ret = libc::getpwuid_r(uid, &mut pwd, cbuf.as_mut_ptr(), BUFFER_LENGTH, &mut res);
    match ret {
        0 => {
            if res == &mut pwd {
                return Ok((pwd, cbuf));
            } else {
                return Err(nix::Error::last());
            }
        },
        _ => {
            return Err(nix::Error::last());
        }
    }
}

pub fn is_uid_valid(uid: libc::uid_t) -> bool {
    uid_to_user(uid).is_ok()
}

pub fn group_to_gid(name: &str) -> Result<libc::gid_t, nix::Error> {
    let cstring = CString::new(name).expect("Could not parse groupname for libc");
    let mut pwd: libc::group = unsafe { std::mem::zeroed() };
    let mut cbuf = [0 as libc::c_char; BUFFER_LENGTH];
    let mut res = ptr::null_mut();

    unsafe {
        let ret = libc::getgrnam_r(cstring.as_ptr(), &mut pwd, cbuf.as_mut_ptr(), BUFFER_LENGTH, &mut res);
        match ret {
            0 => {
                if res == &mut pwd {
                    return Ok((*res).gr_gid);
                } else {
                    return Err(nix::Error::last());
                }
            },
            _ => {
                return Err(nix::Error::last());
            }
        }
    }
}

pub fn gid_to_group(gid: libc::gid_t) -> Result<String, nix::Error> {
    let mut pwd: libc::group = unsafe { std::mem::zeroed() };
    let mut cbuf = [0 as libc::c_char; BUFFER_LENGTH];
    let mut res = ptr::null_mut();

    unsafe {
        let ret = libc::getgrgid_r(gid, &mut pwd, cbuf.as_mut_ptr(), BUFFER_LENGTH, &mut res);
        match ret {
            0 => {
                if res == &mut pwd {
                    return Ok(CStr::from_ptr((*res).gr_name).to_string_lossy().into_owned());
                } else {
                    return Err(nix::Error::last());
                }
            },
            _ => {
                return Err(nix::Error::last());
            }
        }
    }
}

pub fn is_gid_valid(gid: libc::gid_t) -> bool {
    gid_to_group(gid).is_ok()
}

pub fn map_to_errno(error: Error) -> nix::Error {
    match error.raw_os_error() {
        Some(errno) => nix::Error::Sys(nix::errno::Errno::from_i32(errno)),
        _ => nix::Error::Sys(nix::errno::Errno::UnknownErrno),
    }
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
        assert_eq!(Ok("/home/testuser".to_string()), uid_to_homedir(1409));;
    }

    #[test]
    fn invalid_user_homedir_is_reported() {
        assert!(uid_to_homedir(1410).is_err());;
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
        assert!(! is_uid_valid(1410));
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


}