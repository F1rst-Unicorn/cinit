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

//! Overall runtime data structure

mod notify_manager;
mod status_reporter;

use crate::logging;
use crate::runtime::cronjob;
use crate::runtime::dependency_graph;
use crate::runtime::process::ProcessState;
use crate::runtime::process::ProcessType;
use crate::runtime::process_map::ProcessMap;
use crate::util::libc_helpers;
use chrono::prelude::Local;
use log::{debug, error, info, trace, warn};
use nix::sys::epoll;
use nix::sys::signal;
use nix::sys::signalfd;
use nix::sys::wait;
use nix::unistd;
use nix::unistd::Pid;
use std::convert::TryFrom;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;

/// Unique exit code for this module
const EXIT_CODE: i32 = 3;

/// Exit code returned when a child exitted with non-zero exit code
const CHILD_CRASH_EXIT_CODE: i32 = 6;

/// Overall runtime data structure
#[derive(Debug)]
pub struct ProcessManager {
    pub process_map: ProcessMap,

    /// Event loop condition
    pub keep_running: bool,

    pub dependency_manager: dependency_graph::DependencyManager,

    pub cron: cronjob::Cron,

    pub epoll: epoll::Epoll,

    pub signal_fd: signalfd::SignalFd,

    pub status_fd: OwnedFd,

    pub notify_fd: OwnedFd,

    pub exit_code: i32,
}

impl Drop for ProcessManager {
    /// Close all open file descriptors
    fn drop(&mut self) {
        self.deregister_fd_from_epoll(&self.signal_fd);
        self.deregister_fd_from_epoll(&self.status_fd);
        self.deregister_fd_from_epoll(&self.notify_fd);
    }
}

impl ProcessManager {
    /// Set up runtime and run the event loop
    pub fn start(&mut self) -> i32 {
        match self.setup() {
            Err(content) => {
                error!("Failed to register with epoll: {content}");
                return EXIT_CODE;
            }
            _ => {
                debug!("setup successful");
            }
        }

        debug!("Entering poll loop");
        while self.keep_running
            && (self.process_map.has_running_processes() || self.dependency_manager.has_runnables())
        {
            self.spawn_children();
            self.dispatch_epoll();
            self.look_for_finished_children();
        }

        info!("Shutting down");
        while self.process_map.has_running_processes() {
            self.dispatch_epoll();
            self.look_for_finished_children();
        }

        info!("Exiting");
        trace!("Exiting");

        self.exit_code
    }

    /// `wait()` for terminated child processes
    ///
    /// Query for terminated children and update their runtime status.
    fn look_for_finished_children(&mut self) {
        let mut wait_args = wait::WaitPidFlag::empty();
        wait_args.insert(wait::WaitPidFlag::WNOHANG);
        while let Ok(status) = wait::waitpid(Pid::from_raw(-1), Some(wait_args)) {
            match status {
                wait::WaitStatus::Exited(pid, rc) => {
                    debug!("Got signal from child: {status:?}");
                    self.handle_finished_child(pid, rc)
                }
                wait::WaitStatus::Signaled(pid, signal, _) => {
                    debug!("Got signal from child: {status:?}");
                    self.handle_finished_child(pid, signal as i32)
                }
                wait::WaitStatus::StillAlive => {
                    break;
                }
                _ => {
                    debug!("Got unknown result {status:#?}");
                }
            }
        }
    }

    /// Mark a child process as terminated
    ///
    /// Update the child's status. If the child exitted with a non-zero exit code
    /// cinit shuts down.
    ///
    /// The process_map is cleaned off the child's information and the dependency
    /// graph is updated.
    fn handle_finished_child(&mut self, pid: Pid, rc: i32) {
        let child_index_option = self.process_map.process_id_for_pid(pid);

        if child_index_option.is_none() {
            info!("Reaped zombie process {pid} with return code {rc}");
            trace!("Reaped zombie process {pid} with return code {rc}");
            return;
        }

        let child_index = child_index_option.expect("Has been checked above");
        let child_crashed: bool;
        let child = &mut self
            .process_map
            .process_for_pid(pid)
            .expect("Has been checked above");
        let is_cronjob = child.process_type == ProcessType::Cronjob;
        child.state = if rc == 0 {
            child_crashed = false;
            if is_cronjob {
                info!("Child {} has finished and is going to sleep", child.name);
                trace!("Child {} has finished and is going to sleep", child.name);
                ProcessState::Sleeping
            } else {
                info!("Child {} exited successfully", child.name);
                trace!("Child {} exited successfully", child.name);
                ProcessState::Done
            }
        } else {
            error!("Child {} crashed with {}", child.name, rc);
            trace!("Child {} crashed with {}", child.name, rc);
            child_crashed = true;
            self.exit_code = CHILD_CRASH_EXIT_CODE;
            ProcessState::Crashed(rc)
        };

        if child_crashed {
            self.initiate_shutdown(signal::SIGINT);
        }

        self.process_map.deregister_pid(pid);
        if !is_cronjob {
            self.dependency_manager.notify_process_finished(child_index);
        }
    }

    /// Dispatch events from the various file descriptors via `epoll()`
    fn dispatch_epoll(&mut self) {
        let mut event_buffer = [epoll::EpollEvent::empty(); 10];
        let epoll_result = self.epoll.wait(&mut event_buffer, 1000u16);
        match epoll_result {
            Ok(count) => {
                debug!("Got {count} events");
                for event in event_buffer.iter().take(count) {
                    self.handle_event(*event);
                }
            }
            Err(error) => {
                error!("Could not complete epoll: {error:#?}");
            }
        }
    }

    /// Open file descriptors and take on init responsibility
    ///
    /// Signals will be redirected to a file descriptor. The status reporting
    /// UNIX socket is opened. The notify socket for children is opened.
    ///
    /// cinit declares itself as `PR_SET_CHILD_SUBREAPER` to inherit zombies from
    /// its process subtree so it can reap them correctly.
    fn setup(&mut self) -> Result<(), nix::Error> {
        self.setup_epoll_fd()?;
        libc_helpers::prctl_one(libc::PR_SET_CHILD_SUBREAPER, 1)?;
        Ok(())
    }

    /// Set up an `epoll()` file descriptor
    fn setup_epoll_fd(&mut self) -> Result<(), nix::Error> {
        self.epoll.add(
            &self.signal_fd,
            epoll::EpollEvent::new(
                epoll::EpollFlags::EPOLLIN,
                self.signal_fd.as_raw_fd() as u64,
            ),
        )?;
        self.epoll.add(
            &self.status_fd,
            epoll::EpollEvent::new(
                epoll::EpollFlags::EPOLLIN,
                self.status_fd.as_raw_fd() as u64,
            ),
        )?;
        self.epoll.add(
            &self.notify_fd,
            epoll::EpollEvent::new(
                epoll::EpollFlags::EPOLLIN,
                self.notify_fd.as_raw_fd() as u64,
            ),
        )
    }

    /// Handle generic `epoll()` event
    fn handle_event(&mut self, event: epoll::EpollEvent) {
        if event.events().contains(epoll::EpollFlags::EPOLLIN) {
            let fd = event.data() as RawFd;
            if fd == self.signal_fd.as_raw_fd() {
                self.handle_signal();
            } else if fd == self.status_fd.as_raw_fd() {
                self.report_status();
            } else if fd == self.notify_fd.as_raw_fd() {
                self.read_notification();
            } else {
                let fd = unsafe { BorrowedFd::borrow_raw(fd) };
                self.print_child_output(fd);
            }
        } else if event.events().contains(epoll::EpollFlags::EPOLLHUP) {
            let fd = unsafe { BorrowedFd::borrow_raw(event.data() as RawFd) };
            self.deregister_fd_from_epoll(fd);
            self.process_map.deregister_fd(fd)
        } else {
            warn!("Received unknown event");
        }
    }

    /// Handle various signals
    ///
    /// `SIGCHILD` is only logged as child results are yielded via `wait()`.
    ///
    /// `SIGINT`, `SIGQUIT` and `SIGTERM` lead to shutdown.
    fn handle_signal(&mut self) {
        match self.signal_fd.read_signal() {
            Ok(Some(signal)) => {
                match signal::Signal::try_from(signal.ssi_signo as i32).unwrap() {
                    signal @ signal::SIGINT | signal @ signal::SIGQUIT => {
                        self.initiate_shutdown(signal);
                    }
                    signal::SIGTERM => {
                        // Children behave strangely if sent SIGTERM and not
                        // connected to a PTY. Work around this
                        self.initiate_shutdown(signal::SIGINT);
                    }
                    signal::SIGCHLD => {
                        debug!(
                            "Child {} has exited with {}",
                            signal.ssi_pid, signal.ssi_status
                        );
                    }
                    other => {
                        debug!("Received unknown signal: {other:?}");
                    }
                }
            }
            Ok(None) => {
                debug!("No signal received");
            }
            Err(other) => {
                debug!("Received unknown signal: {other:?}");
            }
        }
    }

    /// Shut down the manager
    ///
    /// The event loop is terminated and children are notified to shut down
    fn initiate_shutdown(&mut self, signal: signal::Signal) {
        info!("Received termination signal");
        self.keep_running = false;
        self.signal_children(signal);
    }

    /// Send a signal to all running children
    fn signal_children(&mut self, signal: signal::Signal) {
        info!("Killing children");
        for child in self
            .process_map
            .processes()
            .iter()
            .filter(|s| s.state == ProcessState::Running)
        {
            signal::kill(child.pid, signal).expect("Could not transmit signal to child");
        }
    }

    /// Register a file descriptor at epoll
    fn register_fd_at_epoll(&mut self, fd: &OwnedFd) {
        debug!("Registering fd {}", fd.as_raw_fd());
        let epoll_result = self.epoll.add(
            fd,
            epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, fd.as_raw_fd() as u64),
        );
        if epoll_result.is_err() {
            warn!("Could not unregister fd from epoll");
        }
    }

    /// Remove a file descriptor from epoll
    fn deregister_fd_from_epoll<Fd: AsFd>(&self, fd: Fd) {
        let epoll_result = self.epoll.delete(fd);
        if epoll_result.is_err() {
            warn!("Could not unregister fd from epoll");
        }
    }

    /// Print out child's message reading from its file descriptor
    fn print_child_output(&mut self, fd: BorrowedFd) {
        let mut buffer = [0_u8; 4096];
        let length = unistd::read(fd, &mut buffer);

        if let Ok(length) = length {
            let raw_output = String::from_utf8_lossy(&buffer[..length]);
            let output = raw_output.lines();
            let is_stdout = self.process_map.is_stdout(fd);
            let child_name = &self.process_map.process_for_fd(fd).name;

            for line in output {
                if !line.is_empty() {
                    if is_stdout {
                        logging::stdout::log(child_name, line);
                    } else {
                        logging::stderr::log(child_name, line);
                    }
                }
            }
        }
    }

    /// Check if children are runnable and spawn them
    ///
    /// Look for runnable children in the dependency manager and the cron
    /// scheduler.
    ///
    /// A cron job is only spawned if both its dependencies have been resolved
    /// and the schedule demands it.
    ///
    /// A non-cron process is spawned as soon as its dependencies have been
    /// resolved.
    fn spawn_children(&mut self) {
        while let Some(child_index) = self.dependency_manager.pop_runnable() {
            if self.process_map[child_index].process_type != ProcessType::Cronjob {
                self.spawn_child(child_index);
            }
        }

        while let Some(child_index) = self.cron.pop_runnable(Local::now()) {
            if self.dependency_manager.is_runnable(child_index) {
                self.spawn_child(child_index);
            } else {
                debug!(
                    "Refusing to start cronjob child '{}' because of uncompleted dependencies",
                    self.process_map[child_index].name,
                );
                trace!(
                    "Refusing to start cronjob child '{}' because of uncompleted dependencies",
                    self.process_map[child_index].name,
                );
            }
        }
    }

    /// Spawn the child with the given process id
    ///
    /// The child is spawned unless it is already running which can regularly
    /// happen for cron jobs. The spawned child is indexed via PID, stdout and
    /// stderr file descriptors and is registered at epoll.
    fn spawn_child(&mut self, child_index: usize) {
        let child = &mut self.process_map[child_index];
        if child.state != ProcessState::Blocked && child.state != ProcessState::Sleeping {
            warn!(
                "Refusing to start child '{}' which is currently {}",
                child.name, child.state
            );
            trace!(
                "Refusing to start child '{}' which is currently {}",
                child.name,
                child.state
            );
            return;
        }

        let child = match child.start() {
            Err(child_result) => {
                error!("Failed to spawn child: {child_result}");
                return;
            }
            Ok(v) => v,
        };
        self.register_fd_at_epoll(&child.1);
        self.register_fd_at_epoll(&child.2);
        self.process_map.register_pid(child_index, child.0);
        self.process_map.register_stdout(child_index, child.1);
        self.process_map.register_stderr(child_index, child.2);
    }
}
