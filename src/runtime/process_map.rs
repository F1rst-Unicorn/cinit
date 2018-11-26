use std::collections::HashMap;
use std::os::unix::io::RawFd;
use std::ops::Index;
use std::ops::IndexMut;

use runtime::process::Process;

use nix::unistd::Pid;

#[derive(Debug)]
pub struct ProcessMap {
    processes: Vec<Process>,

    fd_dict: HashMap<RawFd, usize>,

    pid_dict: HashMap<Pid, usize>,
}

impl ProcessMap {
    pub fn from(processes: Vec<Process>) -> ProcessMap {
        ProcessMap {
            processes,
            fd_dict: HashMap::new(),
            pid_dict: HashMap::new(),
        }
    }

    pub fn processes(&self) -> &Vec<Process> {
        &self.processes
    }

    pub fn has_running_processes(&self) -> bool {
        ! self.fd_dict.is_empty() || ! self.pid_dict.is_empty()
    }

    pub fn register_fd(&mut self, process_id: usize, fd: RawFd) {
        self.fd_dict.insert(fd, process_id);
    }

    pub fn deregister_fd(&mut self, fd: &RawFd) {
        self.fd_dict.remove(fd);
    }

    pub fn process_for_fd(&mut self, fd: &RawFd) -> &mut Process {
        let index = *self.fd_dict.get(fd).expect("Requested invalid fd!");
        &mut self.processes[index]
    }

    pub fn register_pid(&mut self, process_id: usize, pid: Pid) {
        self.pid_dict.insert(pid, process_id);
    }

    pub fn deregister_pid(&mut self, pid: &Pid) {
        self.pid_dict.remove(pid);
    }

    pub fn process_id_for_pid(&self, pid: &Pid) -> usize {
        *self.pid_dict.get(pid).expect("Requested invalid pid!")
    }

    pub fn process_for_pid(&mut self, pid: &Pid) -> &mut Process {
        let index = self.process_id_for_pid(pid);
        &mut self.processes[index]
    }
}

impl Index<usize> for ProcessMap {
    type Output = Process;

    fn index(&self, index: usize) ->&Process {
        &self.processes[index]
    }
}

impl IndexMut<usize> for ProcessMap {

    fn index_mut(&mut self, index: usize) ->&mut Process {
        &mut self.processes[index]
    }
}
