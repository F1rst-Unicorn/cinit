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

//! Index over process ids

use std::collections::HashMap;
use std::ops::Index;
use std::ops::IndexMut;
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd, RawFd};

use crate::runtime::process::Process;

use nix::unistd::Pid;

/// Index over process ids
///
/// Owner of all [Processes](Process) at runtime. Each process's id corresponds
/// to the index position inside the `processes` [Vec](std::vec::Vec).
///
/// Maintains indices to map file descriptors and PIDs to their owning processes.
#[derive(Debug)]
pub struct ProcessMap {
    processes: Vec<Process>,

    stdout_dict: HashMap<RawFd, usize>,

    stderr_dict: HashMap<RawFd, usize>,

    pid_dict: HashMap<Pid, usize>,

    fd_dict: HashMap<RawFd, OwnedFd>,
}

impl ProcessMap {
    /// Build from given processes
    pub fn from(processes: Vec<Process>) -> ProcessMap {
        ProcessMap {
            processes,
            stderr_dict: HashMap::new(),
            stdout_dict: HashMap::new(),
            pid_dict: HashMap::new(),
            fd_dict: HashMap::new(),
        }
    }

    /// Get all processes
    pub fn processes(&self) -> &Vec<Process> {
        &self.processes
    }

    /// Check if any process's runtime information is still indexed.
    pub fn has_running_processes(&self) -> bool {
        !self.stdout_dict.is_empty() || !self.stderr_dict.is_empty() || !self.pid_dict.is_empty()
    }

    /// Check the file descriptor is known to the index as stdout
    pub fn is_stdout(&self, fd: BorrowedFd) -> bool {
        self.stdout_dict.contains_key(&fd.as_raw_fd())
    }

    /// Index a new stdout file descriptor for the given process id
    pub fn register_stdout(&mut self, process_id: usize, fd: OwnedFd) {
        self.stdout_dict.insert(fd.as_raw_fd(), process_id);
        self.fd_dict.insert(fd.as_raw_fd(), fd);
    }

    /// Index a new stderr file descriptor for the given process id
    pub fn register_stderr(&mut self, process_id: usize, fd: OwnedFd) {
        self.stderr_dict.insert(fd.as_raw_fd(), process_id);
        self.fd_dict.insert(fd.as_raw_fd(), fd);
    }

    /// Remove file descriptor from the index
    pub fn deregister_fd(&mut self, fd: BorrowedFd) {
        self.stderr_dict.remove(&fd.as_raw_fd());
        self.stdout_dict.remove(&fd.as_raw_fd());
        self.fd_dict.remove(&fd.as_raw_fd());
    }

    /// Get the [Process](Process) owning this file descriptor
    pub fn process_for_fd(&mut self, fd: BorrowedFd) -> &mut Process {
        if let Some(index) = self.stdout_dict.get(&fd.as_raw_fd()) {
            &mut self.processes[*index]
        } else if let Some(index) = self.stderr_dict.get(&fd.as_raw_fd()) {
            &mut self.processes[*index]
        } else {
            panic!("Requested invalid fd");
        }
    }

    /// Register a PID in the index for a process
    pub fn register_pid(&mut self, process_id: usize, pid: Pid) {
        self.pid_dict.insert(pid, process_id);
    }

    /// Deregister a PID from the index
    pub fn deregister_pid(&mut self, pid: Pid) {
        self.pid_dict.remove(&pid);
    }

    /// Look up a PID in the index
    ///
    /// Map a PID into a process id. If the PID is not known to cinit, this PID
    /// is an orphan process adopted by cinit.
    pub fn process_id_for_pid(&self, pid: Pid) -> Option<usize> {
        self.pid_dict.get(&pid).copied()
    }

    /// Look up a PID in the index
    ///
    /// Map a PID into a [Process](Process). If the PID is not known to cinit,
    /// this PID is an orphan process adopted by cinit.
    pub fn process_for_pid(&mut self, pid: Pid) -> Option<&mut Process> {
        let index = self.process_id_for_pid(pid)?;
        Some(&mut self.processes[index])
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
