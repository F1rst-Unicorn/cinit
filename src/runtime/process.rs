use std::ffi::CString;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;
use std::process::exit;

use super::libc_helpers;
use config;

use nix;
use nix::fcntl;
use nix::pty;
use nix::sys::termios;
use nix::sys::stat;
use nix::unistd;
use nix::unistd::fork;
use nix::unistd::Pid;

#[derive(Debug, PartialEq)]
pub enum ProcessState {
    /// The process cannot be started because of dependencies not having
    /// finished yet.
    Blocked,

    /// The process has no more dependencies and can be started
    Ready,

    /// The process is running
    Running,

    /// The process has finished successfully
    Done,

    /// The process has finished unsucessfully
    Crashed,
}

#[derive(Debug)]
pub struct Process {
    pub description: ProcessDescription,

    pub node_info: ProcessNode,
}

impl Process {
    pub fn start(&mut self) -> Result<(Pid, RawFd, RawFd), nix::Error> {
        self.description.start()
    }
}

#[derive(Debug)]
pub struct ProcessDescription {
    pub name: String,

    pub path: String,

    pub args: Vec<CString>,

    pub process_type: config::config::ProcessType,

    pub uid: unistd::Uid,

    pub gid: unistd::Gid,

    pub emulate_pty: bool,

    pub capabilities: Vec<String>,

    pub env: Vec<CString>,

    pub state: ProcessState,

    pub pid: Pid,
}

impl ProcessDescription {
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
            Ok(unistd::ForkResult::Child) => match self.setup_child(stdout.1, stderr.1) {
                Ok(_) => {
                    assert!(false, "exec() was successful but did not replace program");
                    exit(1);
                }
                Err(nix::Error::Sys(errno)) => {
                    error!("Could not exec child {}: {}", self.name, errno.desc());
                    exit(0);
                }
                _ => {
                    error!("Could not exec child {}", self.name);
                    exit(0);
                }
            },
            _ => {
                error!("Forking failed");
                exit(4)
            }
        }
    }

    fn create_std_fds(&self) -> Result<((RawFd, RawFd), (RawFd, RawFd)), nix::Error> {
        let result;
        if self.emulate_pty {
            result = self.create_ptys();
        } else {
            result = self.create_pipes();
        }

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

        println!("Closing duplicated stdout");
        unistd::close(stdout)?;
        println!("Closing duplicated stderr");
        unistd::close(stderr)?;

        println!("Setting gid");
        unistd::setgid(self.gid)?;
        println!("Setting uid");
        unistd::setuid(self.uid)?;

        println!("Doing execve");
        unistd::execve(
            &CString::new(self.path.to_owned()).unwrap(),
            self.args.as_slice(),
            self.env.as_slice(),
        )?;
        Ok(())
    }

    fn create_ptys(&self) -> Result<((RawFd, RawFd), (RawFd, RawFd)), nix::Error> {
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
            ioctl_result = libc_helpers::get_terminal_size(stdin, &mut winsize);
        }

        if tcget_result.is_err() {
            info!("Could not get terminal flags");
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

        unistd::chown(stdout_name.to_bytes(), Some(self.uid), Some(self.gid))?;
        unistd::chown(stderr_name.to_bytes(), Some(self.uid), Some(self.gid))?;

        let mut mode = stat::Mode::empty();
        mode.insert(stat::Mode::S_IRUSR);
        mode.insert(stat::Mode::S_IWUSR);
        mode.insert(stat::Mode::S_IWGRP);
        stat::fchmod(stdout.slave, mode)?;
        stat::fchmod(stderr.slave, mode)?;

        info!("Pseudo terminals created");
        Ok(((stdout.master, stdout.slave), (stderr.master, stderr.slave)))
    }

    fn create_pipes(&self) -> Result<((RawFd, RawFd), (RawFd, RawFd)), nix::Error> {
        let stdout = unistd::pipe().unwrap();
        let stderr = unistd::pipe().unwrap();
        Ok((stdout, stderr))
    }
}

/// Process information relevant for dependency resolution
/// via ongoing topological sorting
#[derive(Debug)]
pub struct ProcessNode {
    pub before: Vec<usize>,

    pub predecessor_count: usize,
}
