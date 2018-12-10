use std::ffi::CString;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;

use crate::util::libc_helpers;
use crate::util::libc_helpers::map_to_errno;
use crate::util::libc_helpers::get_terminal_size;

use nix;
use nix::fcntl;
use nix::pty;
use nix::sys::stat;
use nix::sys::termios;
use nix::unistd;
use nix::unistd::fork;
use nix::unistd::Pid;

use capabilities::Capabilities;
use capabilities::Capability;
use capabilities::Flag;

use log::{trace, debug, info, error};

const EXIT_CODE: i32 = 4;

#[derive(Debug, PartialEq)]
pub enum ProcessState {
    /// The process cannot be started because of dependencies not having
    /// finished yet.
    Blocked,

    /// The process is a cronjob and waits for its timer to be triggered
    Sleeping,

    /// The process is running
    Running,

    /// The process has finished successfully
    Done,

    /// The process has finished unsucessfully
    Crashed,
}

type Pipe = (RawFd, RawFd);

impl Display for ProcessState {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        let message = match self {
            ProcessState::Blocked => "blocked",
            ProcessState::Sleeping => "sleeping",
            ProcessState::Running => "running",
            ProcessState::Done => "done",
            ProcessState::Crashed => "crashed",
        };
        write!(f, "{}", message)
    }
}

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

    pub pid: Pid,
}

impl Process {
    pub fn start(&mut self) -> Result<(Pid, RawFd, RawFd), nix::Error> {
        info!("Starting {}", self.name);

        let (stdout, stderr) = self.create_std_fds()?;

        let fork_result = fork();

        match fork_result {
            Ok(unistd::ForkResult::Parent { child: child_pid }) => {
                trace!("Started child {}", self.name);
                info!("Started child {}", child_pid);
                self.state = ProcessState::Running;
                self.pid = child_pid;
                unistd::close(stdout.1)?;
                unistd::close(stderr.1)?;
                Ok((child_pid, stdout.0, stderr.0))
            }
            Ok(unistd::ForkResult::Child) => {
                match self.setup_child(stdout.1, stderr.1) {
                    Ok(_) => {
                        assert!(false, "exec() was successful but did not replace program");
                        // Intentionally other exit code as this is the child
                        exit(1);
                    }
                    Err(nix::Error::Sys(errno)) => {
                        println!("Could not exec child {}: {}", self.name, errno.desc());
                        exit(EXIT_CODE);
                    }
                    _ => {
                        println!("Could not exec child {}", self.name);
                        exit(EXIT_CODE);
                    }
                }
            }
            _ => {
                error!("Forking failed");
                exit(EXIT_CODE)
            }
        }
    }

    fn create_std_fds(&self) -> Result<(Pipe, Pipe), nix::Error> {
        let result = if self.emulate_pty {
            self.create_ptys()
        } else {
            self.create_pipes()
        };

        if result.is_ok() {
            let fds = result.unwrap();
            fcntl::fcntl(
                (fds.0).0,
                fcntl::FcntlArg::F_SETFD(fcntl::FdFlag::FD_CLOEXEC),
            )?;
            fcntl::fcntl(
                (fds.1).0,
                fcntl::FcntlArg::F_SETFD(fcntl::FdFlag::FD_CLOEXEC),
            )?;
        }
        result
    }

    fn setup_child(&mut self, stdout: RawFd, stderr: RawFd) -> Result<(), nix::Error> {
        while let Err(_) = unistd::dup2(stdout, std::io::stdout().as_raw_fd()) {}
        while let Err(_) = unistd::dup2(stderr, std::io::stderr().as_raw_fd()) {}

        unistd::close(stdout)?;
        unistd::close(stderr)?;

        std::env::set_current_dir(&self.workdir).map_err(|e| match e.raw_os_error() {
            None => nix::Error::UnsupportedOperation,
            Some(code) => nix::Error::Sys(nix::errno::Errno::from_i32(code)),
        })?;

        self.set_user_and_caps()?;

        unistd::execvpe(
            &CString::new(self.path.to_owned()).unwrap(),
            self.args.as_slice(),
            self.env.as_slice(),
        )?;
        Ok(())
    }

    fn set_user_and_caps(&mut self) -> Result<(), nix::Error> {
        let mut temporary_caps = Capabilities::new().map_err(map_to_errno)?;
        let mut actual_caps = Capabilities::new().map_err(map_to_errno)?;
        let flags = [
            Capability::CAP_SETUID,
            Capability::CAP_SETGID,
            Capability::CAP_SETPCAP,
            Capability::CAP_SETFCAP,
        ];
        temporary_caps.update(&flags, Flag::Permitted, true);
        temporary_caps.update(&flags, Flag::Effective, true);
        temporary_caps.update(&flags, Flag::Inheritable, true);
        for raw_cap in &self.capabilities {
            let new_cap = Capability::from_str(raw_cap);
            match new_cap {
                Ok(cap) => {
                    actual_caps.update(&[cap], Flag::Permitted, true);
                    actual_caps.update(&[cap], Flag::Effective, true);
                    actual_caps.update(&[cap], Flag::Inheritable, true);
                    temporary_caps.update(&[cap], Flag::Permitted, true);
                    temporary_caps.update(&[cap], Flag::Effective, true);
                    temporary_caps.update(&[cap], Flag::Inheritable, true);
                }
                _ => {
                    println!("Failed to set {}", raw_cap);
                }
            }
        }

        temporary_caps.apply().map_err(map_to_errno)?;
        libc_helpers::prctl_one(libc::PR_SET_KEEPCAPS, 1)?;
        unistd::setgid(self.gid)?;
        unistd::setuid(self.uid)?;
        libc_helpers::prctl_one(libc::PR_SET_KEEPCAPS, 0)?;
        temporary_caps.apply().map_err(map_to_errno)?;

        libc_helpers::prctl_four(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL as libc::c_ulong,
            0,
            0,
            0,
        )?;
        for raw_cap in &self.capabilities {
            let new_cap = Capability::from_str(raw_cap);
            match new_cap {
                Ok(cap) => libc_helpers::prctl_four(
                    libc::PR_CAP_AMBIENT,
                    libc::PR_CAP_AMBIENT_RAISE as libc::c_ulong,
                    cap as libc::c_ulong,
                    0,
                    0,
                )?,
                _ => {
                    println!("Failed to set {}", raw_cap);
                }
            }
        }

        actual_caps.apply().map_err(map_to_errno)?;
        Ok(())
    }

    fn create_ptys(&self) -> Result<(Pipe, Pipe), nix::Error> {
        let stdin = std::io::stdin().as_raw_fd();
        let mut tcget_result = termios::tcgetattr(stdin);
        let ioctl_result: Result<libc::c_int, nix::Error>;
        let mut winsize = pty::Winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        unsafe {
            ioctl_result = get_terminal_size(stdin, &mut winsize);
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

        let stdout_name = libc_helpers::ttyname(stdout.slave)?;
        let stderr_name = libc_helpers::ttyname(stderr.slave)?;

        unistd::chown(stdout_name.as_bytes(), Some(self.uid), Some(self.gid))?;
        unistd::chown(stderr_name.as_bytes(), Some(self.uid), Some(self.gid))?;

        let mut mode = stat::Mode::empty();
        mode.insert(stat::Mode::S_IRUSR);
        mode.insert(stat::Mode::S_IWUSR);
        mode.insert(stat::Mode::S_IWGRP);
        stat::fchmod(stdout.slave, mode)?;
        stat::fchmod(stderr.slave, mode)?;

        info!("Pseudo terminals created");
        Ok(((stdout.master, stdout.slave), (stderr.master, stderr.slave)))
    }

    fn create_pipes(&self) -> Result<(Pipe, Pipe), nix::Error> {
        let stdout = unistd::pipe().unwrap();
        let stderr = unistd::pipe().unwrap();
        Ok((stdout, stderr))
    }
}
