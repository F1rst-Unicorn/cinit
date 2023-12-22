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

//! Data and behaviour of a single process

use crate::util::libc_helpers;
use crate::util::libc_helpers::get_terminal_size;
use caps::clear as clear_capabilities;
use caps::set as apply_capabilities;
use caps::CapSet;
use caps::Capability;
use caps::CapsHashSet;
use log::{debug, error, info, trace, warn};
use nix::fcntl;
use nix::pty;
use nix::sys::signal;
use nix::sys::stat;
use nix::sys::termios;
use nix::unistd;
use nix::unistd::fork;
use nix::unistd::Pid;
use nix::Error;
use std::ffi::CStr;
use std::ffi::CString;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;

/// Unique exit code for this module
///
/// Raised by child processes where setup failed or by cinit if forking has
/// failed.
const EXIT_CODE: i32 = 4;

/// Runtime process type
///
/// Runtime pendant to [configuration ProcessType](crate::config::ProcessType)
/// without unneeded contained parameters.
#[derive(Debug, PartialEq, Eq)]
pub enum ProcessType {
    Oneshot,

    Notify,

    Cronjob,
}

/// States a process can take on
#[derive(Debug, PartialEq, Eq)]
pub enum ProcessState {
    /// The process cannot be started because of dependencies not having
    /// finished yet.
    Blocked,

    /// The process is a cronjob and waits for its timer to be triggered
    Sleeping,

    /// The process is a notify, has been started by cinit and has not told cinit
    /// that it has started
    Starting,

    /// The process is running. Set automatically for oneshot and by the process
    /// itself for notify
    Running,

    /// The process is a notify and has told cinit that it is stopping
    Stopping,

    /// The process has finished successfully
    Done,

    /// The process has finished unsucessfully
    Crashed(i32),
}

type Pipe = (OwnedFd, OwnedFd);

impl Display for ProcessState {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        let message = match self {
            ProcessState::Blocked => "blocked",
            ProcessState::Sleeping => "sleeping",
            ProcessState::Starting => "starting",
            ProcessState::Running => "running",
            ProcessState::Stopping => "stopping",
            ProcessState::Done => "done",
            ProcessState::Crashed(_) => "crashed",
        };
        write!(f, "{}", message)
    }
}

/// Runtime information of a single process
///
/// Store information needed during the entire lifetime of the process
#[derive(Debug)]
pub struct Process {
    pub name: String,

    pub path: String,

    pub args: Vec<CString>,

    pub workdir: PathBuf,

    pub uid: unistd::Uid,

    pub gid: unistd::Gid,

    pub emulate_pty: bool,

    pub capabilities: Vec<String>,

    pub env: Vec<CString>,

    pub state: ProcessState,

    pub process_type: ProcessType,

    pub pid: Pid,

    pub status: String,
}

impl Process {
    /// Start a new [Process](Process) by forking
    ///
    /// Fork off the process returning its PID, `stdout`, and `stderr` file
    /// descriptors. The child process will configure according to the
    /// [ProcessConfig](crate::config::ProcessConfig) and then perform an `exec`.
    pub fn start(&mut self) -> Result<(Pid, OwnedFd, OwnedFd), Error> {
        info!("Starting {}", self.name);

        let (stdout, stderr) = self.create_std_fds()?;

        let fork_result = unsafe {
            // We are in a single-threaded program, so this unsafe call is ok
            // https://docs.rs/nix/0.19.0/nix/unistd/fn.fork.html#safety
            fork()
        };

        match fork_result {
            Ok(unistd::ForkResult::Parent { child: child_pid }) => {
                trace!("Started child {}", self.name);
                info!("Started child {}", child_pid);
                self.state = match self.process_type {
                    ProcessType::Notify => ProcessState::Starting,
                    _ => ProcessState::Running,
                };
                self.pid = child_pid;
                drop(stdout.1);
                drop(stderr.1);
                Ok((child_pid, stdout.0, stderr.0))
            }
            Ok(unistd::ForkResult::Child) => match self.setup_child(stdout.1, stderr.1) {
                Ok(_) => {
                    panic!("exec() was successful but did not replace program");
                }
                Err(errno) => {
                    println!("Could not exec child {}: {}", self.name, errno.desc());
                    // child exit
                    exit(EXIT_CODE);
                }
            },
            _ => {
                error!("Forking failed");
                Err(Error::EINVAL)
            }
        }
    }

    /// Handle information received from the `notify` socket.
    ///
    /// Keys of interest are:
    ///
    /// * `READY`
    /// * `STOPPING`
    /// * `STATUS`
    /// * `MAINPID`
    pub fn handle_notification(&mut self, key: &str, value: &str) {
        match key {
            "READY" => {
                if value != "1" {
                    warn!("Expected READY=1 but value was '{}'", value);
                    return;
                }

                if self.state == ProcessState::Starting {
                    info!("child {} has started successfully", self.name);
                    trace!("child {} has started successfully", self.name);
                    self.state = ProcessState::Running;
                } else {
                    debug!(
                        "child {} in {} state has notified about startup",
                        self.name, self.state
                    );
                }
            }
            "STOPPING" => {
                if value != "1" {
                    warn!("Expected STOPPING=1 but value was '{}'", value);
                    return;
                }

                info!("child {} is shutting down", self.name);
                trace!("child {} is shutting down", self.name);
                self.state = ProcessState::Stopping;
            }
            "STATUS" => {
                trace!("child {}: {}", self.name, value);
                self.status = value.to_string();
            }
            "MAINPID" => {
                let pid_result = value.parse::<libc::pid_t>();
                if let Err(e) = pid_result {
                    warn!("could not parse new main pid '{}': {}", value, e);
                    return;
                }

                let pid = Pid::from_raw(pid_result.unwrap());

                if pid != self.pid {
                    info!(
                        "child {} main pid is changed from {} to {}",
                        self.name, self.pid, pid
                    );
                    trace!(
                        "child {} main pid is changed from {} to {}",
                        self.name,
                        self.pid,
                        pid
                    );
                }

                self.pid = pid;
            }
            _ => {}
        };
    }

    /// Create file descriptors for stdout and stderr
    ///
    /// Either create plain pipes or pty-emulating pipes, depending on
    /// [`emulate_pty`](crate::config::ProcessConfig::emulate_pty).
    ///
    /// The cinit parts of the pipes are closed on exec, so the child cannot
    /// abuse them.
    fn create_std_fds(&self) -> Result<(Pipe, Pipe), Error> {
        let result = if self.emulate_pty {
            self.create_ptys()
        } else {
            self.create_pipes()
        };

        if let Ok(fds) = &result {
            fcntl::fcntl(
                fds.0 .0.as_raw_fd(),
                fcntl::FcntlArg::F_SETFD(fcntl::FdFlag::FD_CLOEXEC),
            )?;
            fcntl::fcntl(
                fds.1 .0.as_raw_fd(),
                fcntl::FcntlArg::F_SETFD(fcntl::FdFlag::FD_CLOEXEC),
            )?;
        }
        result
    }

    /// Run pre-exec setup and do exec.
    ///
    /// This call will not return but instead `exec`!
    ///
    /// The existing stdout and stderr file descriptors inherited from cinit are
    /// replaced by the parameters.
    ///
    /// cinit's `sigprocmask` is reverted to not mask any signals.
    fn setup_child(&mut self, stdout: OwnedFd, stderr: OwnedFd) -> Result<(), Error> {
        while unistd::dup2(stdout.as_raw_fd(), std::io::stdout().as_raw_fd()).is_err() {}
        while unistd::dup2(stderr.as_raw_fd(), std::io::stderr().as_raw_fd()).is_err() {}

        let signals = signal::SigSet::empty();
        signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&signals), None)?;

        drop(stdout);
        drop(stderr);

        std::env::set_current_dir(&self.workdir).map_err(|e| match e.raw_os_error() {
            None => Error::EINVAL,
            Some(code) => nix::errno::Errno::from_i32(code),
        })?;

        self.set_user_and_caps()?;

        unistd::execvpe(
            &CString::new(self.path.to_owned()).unwrap(),
            self.args
                .iter()
                .map(CString::as_c_str)
                .collect::<Vec<&CStr>>()
                .as_slice(),
            self.env
                .iter()
                .map(CString::as_c_str)
                .collect::<Vec<&CStr>>()
                .as_slice(),
        )?;
        Ok(())
    }

    /// Set security features of the child process
    ///
    /// Switch to the specified UNIX user and group.
    ///
    /// Configure ambient capabilities.
    ///
    /// These two operations have to happen jointly due to security confinements:
    ///
    /// * First temporary capabilities are added to the permitted set to allow
    ///   transferring them across a uid/gid change.
    ///
    /// * Then the uid/gid is changed using `PR_SET_KEEPCAPS` (see `man 7
    ///   capabilities`)
    ///
    /// * Set the [configured
    ///   capabilities](crate::config::ProcessConfig::capabilities) as the
    ///   unprivileged user.
    fn set_user_and_caps(&mut self) -> Result<(), Error> {
        let mut actual_caps = CapsHashSet::default();

        for raw_cap in &self.capabilities {
            let new_cap = Capability::from_str(raw_cap);
            match new_cap {
                Ok(cap) => {
                    actual_caps.insert(cap);
                }
                _ => {
                    println!("Failed to set {}", raw_cap);
                }
            }
        }

        let mut temporary_caps = actual_caps.clone();
        temporary_caps.insert(Capability::CAP_SETUID);
        temporary_caps.insert(Capability::CAP_SETGID);
        temporary_caps.insert(Capability::CAP_SETPCAP);
        temporary_caps.insert(Capability::CAP_SETFCAP);

        apply_capabilities(None, CapSet::Inheritable, &temporary_caps).map_err(map_to_errno)?;
        apply_capabilities(None, CapSet::Effective, &temporary_caps).map_err(map_to_errno)?;
        libc_helpers::prctl_one(libc::PR_SET_KEEPCAPS, 1)?;
        unistd::setgid(self.gid)?;
        unistd::setgroups(&[self.gid])?;
        unistd::setuid(self.uid)?;
        libc_helpers::prctl_one(libc::PR_SET_KEEPCAPS, 0)?;
        apply_capabilities(None, CapSet::Inheritable, &temporary_caps).map_err(map_to_errno)?;
        apply_capabilities(None, CapSet::Effective, &temporary_caps).map_err(map_to_errno)?;
        clear_capabilities(None, CapSet::Ambient).map_err(map_to_errno)?;

        apply_capabilities(None, CapSet::Ambient, &actual_caps).map_err(map_to_errno)?;
        apply_capabilities(None, CapSet::Inheritable, &actual_caps).map_err(map_to_errno)?;
        apply_capabilities(None, CapSet::Effective, &actual_caps).map_err(map_to_errno)?;
        Ok(())
    }

    fn create_ptys(&self) -> Result<(Pipe, Pipe), Error> {
        let stdin = std::io::stdin();
        let mut tcget_result = termios::tcgetattr(&stdin);
        let ioctl_result: Result<libc::c_int, Error>;
        let mut winsize = pty::Winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        unsafe {
            ioctl_result = get_terminal_size(stdin.as_raw_fd(), &mut winsize);
        }

        if tcget_result.is_err() {
            debug!("Could not get terminal flags");
        } else {
            let mut termios = tcget_result.unwrap();
            termios.input_flags = termios::InputFlags::empty();
            termios.input_flags.insert(
                termios::InputFlags::BRKINT
                    | termios::InputFlags::ICRNL
                    | termios::InputFlags::INPCK
                    | termios::InputFlags::ISTRIP
                    | termios::InputFlags::IXON,
            );
            termios.output_flags = termios::OutputFlags::empty();
            termios.output_flags.insert(termios::OutputFlags::OPOST);
            termios.local_flags = termios::LocalFlags::empty();
            termios.local_flags.insert(
                termios::LocalFlags::ECHO
                    | termios::LocalFlags::ICANON
                    | termios::LocalFlags::IEXTEN
                    | termios::LocalFlags::ISIG,
            );
            tcget_result = Ok(termios);
        }

        if ioctl_result.is_err() {
            debug!("Not running inside tty, using sane defaults");
            winsize = pty::Winsize {
                ws_row: 24,
                ws_col: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
        }

        let stdout = pty::openpty(Some(&winsize), &tcget_result.clone().ok())?;
        let stderr = pty::openpty(Some(&winsize), &tcget_result.ok())?;

        let stdout_name = libc_helpers::ttyname(stdout.slave.as_raw_fd())?;
        let stderr_name = libc_helpers::ttyname(stderr.slave.as_raw_fd())?;

        unistd::chown(stdout_name.as_bytes(), Some(self.uid), Some(self.gid))?;
        unistd::chown(stderr_name.as_bytes(), Some(self.uid), Some(self.gid))?;

        let mut mode = stat::Mode::empty();
        mode.insert(stat::Mode::S_IRUSR);
        mode.insert(stat::Mode::S_IWUSR);
        mode.insert(stat::Mode::S_IWGRP);
        stat::fchmod(stdout.slave.as_raw_fd(), mode)?;
        stat::fchmod(stderr.slave.as_raw_fd(), mode)?;

        info!("Pseudo terminals created");
        Ok(((stdout.master, stdout.slave), (stderr.master, stderr.slave)))
    }

    fn create_pipes(&self) -> Result<(Pipe, Pipe), Error> {
        let stdout = unistd::pipe()?;
        let stderr = unistd::pipe()?;
        Ok(unsafe {
            (
                (
                    OwnedFd::from_raw_fd(stdout.0),
                    OwnedFd::from_raw_fd(stdout.1),
                ),
                (
                    OwnedFd::from_raw_fd(stderr.0),
                    OwnedFd::from_raw_fd(stderr.1),
                ),
            )
        })
    }
}

fn map_to_errno(e: caps::errors::CapsError) -> Error {
    println!("capability error: {}", e);
    Error::EINVAL
}
