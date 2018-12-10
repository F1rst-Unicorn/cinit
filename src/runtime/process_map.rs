use std::collections::HashMap;
use std::ops::Index;
use std::ops::IndexMut;
use std::os::unix::io::RawFd;

use crate::runtime::process::Process;

use nix::unistd::Pid;

#[derive(Debug, Clone, Copy)]
pub enum StreamType {
    Stdout,
    Stderr,
}

#[derive(Debug)]
pub struct ProcessMap {
    processes: Vec<Process>,

    stdout_dict: HashMap<RawFd, usize>,

    stderr_dict: HashMap<RawFd, usize>,

    pid_dict: HashMap<Pid, usize>,
}

impl ProcessMap {
    pub fn from(processes: Vec<Process>) -> ProcessMap {
        ProcessMap {
            processes,
            stderr_dict: HashMap::new(),
            stdout_dict: HashMap::new(),
            pid_dict: HashMap::new(),
        }
    }

    pub fn processes(&self) -> &Vec<Process> {
        &self.processes
    }

    pub fn has_running_processes(&self) -> bool {
        !self.stdout_dict.is_empty() || !self.stderr_dict.is_empty() || !self.pid_dict.is_empty()
    }

    pub fn is_stdout(&self, fd: RawFd) -> bool {
        self.stdout_dict.contains_key(&fd)
    }

    pub fn register_stdout(&mut self, process_id: usize, fd: RawFd) {
        self.register_fd(process_id, fd, StreamType::Stdout)
    }

    pub fn register_stderr(&mut self, process_id: usize, fd: RawFd) {
        self.register_fd(process_id, fd, StreamType::Stderr)
    }

    pub fn register_fd(&mut self, process_id: usize, fd: RawFd, kind: StreamType) {
        match kind {
            StreamType::Stdout => {
                self.stdout_dict.insert(fd, process_id);
            }
            StreamType::Stderr => {
                self.stderr_dict.insert(fd, process_id);
            }
        }
    }

    pub fn deregister_fd(&mut self, fd: RawFd) {
        self.stderr_dict.remove(&fd);
        self.stdout_dict.remove(&fd);
    }

    pub fn process_for_fd(&mut self, fd: RawFd) -> &mut Process {
        if let Some(index) = self.stdout_dict.get(&fd) {
            &mut self.processes[*index]
        } else if let Some(index) = self.stderr_dict.get(&fd) {
            &mut self.processes[*index]
        } else {
            panic!("Requested invalid fd");
        }
    }

    pub fn register_pid(&mut self, process_id: usize, pid: Pid) {
        self.pid_dict.insert(pid, process_id);
    }

    pub fn deregister_pid(&mut self, pid: Pid) {
        self.pid_dict.remove(&pid);
    }

    pub fn process_id_for_pid(&self, pid: Pid) -> usize {
        *self.pid_dict.get(&pid).expect("Requested invalid pid!")
    }

    pub fn process_for_pid(&mut self, pid: Pid) -> &mut Process {
        let index = self.process_id_for_pid(pid);
        &mut self.processes[index]
    }
}

impl Index<usize> for ProcessMap {
    type Output = Process;

    fn index(&self, index: usize) -> &Process {
        &self.processes[index]
    }
}

impl IndexMut<usize> for ProcessMap {
    fn index_mut(&mut self, index: usize) -> &mut Process {
        &mut self.processes[index]
    }
}
