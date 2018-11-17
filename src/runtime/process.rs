use std::process::exit;
use std::ffi::CString;
use std::os::unix::io::RawFd;
use std::os::unix::io::AsRawFd;

use config;

use nix::unistd;
use nix::unistd::Pid;
use nix::unistd::fork;
use nix::fcntl;

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

    pub uid: u32,

    pub gid: u32,

    pub emulate_pty: bool,

    pub capabilities: Vec<String>,

    pub env: Vec<CString>,

    pub state: ProcessState,

    pub pid: Pid,
}

impl ProcessDescription {

    pub fn start(&mut self) -> Result<(Pid, RawFd, RawFd), nix::Error> {

        info!("Starting {}", self.name);

        let stdout = unistd::pipe().unwrap();
        let stderr = unistd::pipe().unwrap();
        fcntl::fcntl(stdout.0, fcntl::FcntlArg::F_SETFD(fcntl::FdFlag::FD_CLOEXEC))?;
        fcntl::fcntl(stderr.0, fcntl::FcntlArg::F_SETFD(fcntl::FdFlag::FD_CLOEXEC))?;

        let fork_result = fork();

        match fork_result {
            Ok(unistd::ForkResult::Parent {child: child_pid}) => {
                info!("Started child {}", child_pid);
                self.state = ProcessState::Running;
                self.pid = child_pid;
                unistd::close(stdout.1)?;
                unistd::close(stderr.1)?;
                Ok((child_pid, stdout.0, stderr.0))
            },
            Ok(unistd::ForkResult::Child) => {
                match self.setup_child(stdout.1, stderr.1) {
                    Ok(_) => {
                        assert!(false, "exec() was successful but did not replace program");
                        exit(1);
                    }
                    Err(nix::Error::Sys(errno)) => {
                        error!("Could not exec child {}: {}", self.name, errno.desc());
                        exit(0);
                    },
                    _ => {
                        error!("Could not exec child {}", self.name);
                        exit(0);
                    }
                }
            },
            _ => {
                error!("Forking failed");
                exit(4)
            }
        }
    }

    fn setup_child(&mut self, stdout: RawFd, stderr: RawFd) -> Result<(), nix::Error> {
        while let Err(_) = unistd::dup2(stdout, std::io::stdout().as_raw_fd()) {}
        while let Err(_) = unistd::dup2(stderr, std::io::stderr().as_raw_fd()) {}

        unistd::setuid(unistd::Uid::from_raw(self.uid))?;
        unistd::setgid(unistd::Gid::from_raw(self.gid))?;

        unistd::execve(&CString::new(self.path.to_owned()).unwrap(),
                       self.args.as_slice(),
                       self.env.as_slice())?;
        Ok(())
    }
}

/// Process information relevant for dependency resolution
/// via ongoing topological sorting
#[derive(Debug)]
pub struct ProcessNode {
    pub before: Vec<usize>,

    pub predecessor_count: usize,
}
