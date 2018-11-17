use std::collections::HashMap;
use std::collections::VecDeque;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;
use std::process::exit;

use runtime::process::{Process, ProcessState};

use nix::sys::epoll;
use nix::sys::signal;
use nix::sys::signalfd;
use nix::sys::wait;
use nix::unistd;
use nix::unistd::Pid;

#[derive(Debug)]
pub struct ProcessManager {
    pub processes: Vec<Process>,

    pub name_dict: HashMap<String, usize>,

    pub fd_dict: HashMap<RawFd, usize>,

    pub pid_dict: HashMap<Pid, usize>,

    pub keep_running: bool,

    pub runnable: VecDeque<usize>,

    pub epoll_file: RawFd,

    pub signal_fd: signalfd::SignalFd,
}

impl ProcessManager {
    pub fn start(&mut self) {
        match self.setup() {
            Err(content) => {
                error!("Failed to register with epoll: {}", content);
                exit(3);
            }
            _ => {
                debug!("setup successful");
            }
        }

        debug!("Entering poll loop");
        while self.keep_running && (self.pid_dict.len() != 0 || self.runnable.len() != 0) {
            self.kick_off_children();
            self.dispatch_epoll();
            self.look_for_finished_children(false);
        }

        self.look_for_finished_children(true);
        info!("Exiting");
    }

    fn look_for_finished_children(&mut self, wait_for_all: bool) {
        let mut wait_args = wait::WaitPidFlag::empty();
        if !wait_for_all {
            wait_args.insert(wait::WaitPidFlag::WNOHANG);
        }
        while let Ok(status) = wait::waitpid(Pid::from_raw(-1), Some(wait_args)) {
            match status {
                wait::WaitStatus::Exited(pid, rc) => {
                    debug!("Got signal from child: {:?}", status);
                    self.handle_finished_child(&pid, rc)
                }
                wait::WaitStatus::Signaled(pid, signal, _) => {
                    debug!("Got signal from child: {:?}", status);
                    self.handle_finished_child(&pid, signal as i32)
                }
                wait::WaitStatus::StillAlive => {
                    break;
                }
                _ => {
                    debug!("Got unknown result {:#?}", status);
                }
            }
        }
    }

    fn handle_finished_child(&mut self, pid: &Pid, rc: i32) {
        let child_index = *self.pid_dict.get(&pid).expect("PID not found");
        {
            let child = &mut self.processes[child_index];
            child.description.state = if rc == 0 {
                info!("Child {} exited successfully", child.description.name);
                ProcessState::Done
            } else {
                warn!("Child {} crashed with {}", child.description.name, rc);
                ProcessState::Crashed
            }
        }
        self.pid_dict.remove(pid);

        for successor_index in self.processes[child_index].node_info.before.clone() {
            let mut successor = &mut self.processes[successor_index];
            successor.node_info.predecessor_count -= 1;
            if successor.node_info.predecessor_count == 0 {
                self.runnable.push_back(successor_index);
            }
        }
    }

    fn dispatch_epoll(&mut self) {
        let mut event_buffer = [epoll::EpollEvent::empty(); 10];
        let epoll_result = epoll::epoll_wait(self.epoll_file, &mut event_buffer, 1000);
        match epoll_result {
            Ok(count) => {
                debug!("Got {} events", count);
                for i in 0..count {
                    let event = event_buffer[i];
                    self.handle_event(event);
                }
            }
            Err(error) => {
                error!("Could not complete epoll: {:#?}", error);
            }
        }
    }

    fn setup(&mut self) -> Result<(), nix::Error> {
        self.signal_fd = ProcessManager::setup_signal_handler()?;
        self.epoll_file = self.setup_epoll_fd()?;
        Ok(())
    }

    fn setup_signal_handler() -> Result<signalfd::SignalFd, nix::Error> {
        let mut signals = signalfd::SigSet::empty();
        signals.add(signalfd::signal::SIGCHLD);
        signals.add(signalfd::signal::SIGINT);
        signals.add(signalfd::signal::SIGTERM);
        signals.add(signalfd::signal::SIGQUIT);
        signal::sigprocmask(signal::SigmaskHow::SIG_BLOCK, Some(&signals), None)?;
        signalfd::SignalFd::with_flags(&signals, signalfd::SfdFlags::SFD_CLOEXEC)
    }

    fn setup_epoll_fd(&mut self) -> Result<RawFd, nix::Error> {
        let epoll_fd = epoll::epoll_create1(epoll::EpollCreateFlags::EPOLL_CLOEXEC)?;
        epoll::epoll_ctl(
            epoll_fd,
            epoll::EpollOp::EpollCtlAdd,
            self.signal_fd.as_raw_fd(),
            &mut epoll::EpollEvent::new(
                epoll::EpollFlags::EPOLLIN,
                self.signal_fd.as_raw_fd() as u64,
            ),
        )?;
        Ok(epoll_fd)
    }

    fn handle_event(&mut self, event: epoll::EpollEvent) {
        if event.events().contains(epoll::EpollFlags::EPOLLIN) {
            let fd = event.data() as RawFd;
            if fd == self.signal_fd.as_raw_fd() {
                self.handle_signal();
            } else {
                self.handle_child_output(fd);
            }
        } else if event.events().contains(epoll::EpollFlags::EPOLLHUP) {
            self.deregister_fd(event.data() as RawFd);
        } else {
            warn!("Received unknown event");
        }
    }

    fn handle_signal(&mut self) {
        match self.signal_fd.read_signal() {
            Ok(Some(signal)) => {
                match signal::Signal::from_c_int(signal.ssi_signo as i32).unwrap() {
                    signal::SIGINT | signal::SIGTERM | signal::SIGQUIT => {
                        info!("Received termination signal, killing children");
                        self.keep_running = false;
                        for child in &self.processes {
                            if child.description.state == ProcessState::Running {
                                signal::kill(child.description.pid, signal::SIGTERM)
                                    .expect("Could not transmit signal to child");
                            }
                        }
                    }
                    signal::SIGCHLD => {
                        debug!(
                            "Child {} has exited with {}",
                            signal.ssi_pid, signal.ssi_status
                        );
                    }
                    other => {
                        debug!("Received unknown signal: {:?}", other);
                    }
                }
            }
            Ok(None) => {
                debug!("No signal received");
            }
            Err(other) => {
                debug!("Received unknown signal: {:?}", other);
            }
        }
    }

    fn register_fd(&mut self, fd: RawFd, child: usize) {
        debug!("Registering fd {}", fd);
        let epoll_result = epoll::epoll_ctl(
            self.epoll_file,
            epoll::EpollOp::EpollCtlAdd,
            fd,
            &mut epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, fd as u64),
        );
        if epoll_result.is_err() {
            warn!("Could not unregister fd from epoll");
        }
        self.fd_dict.insert(fd, child);
        debug!("{:?}", self.fd_dict);
    }

    fn deregister_fd(&mut self, fd: RawFd) {
        debug!("Deregistering fd {}", fd);
        let epoll_result = epoll::epoll_ctl(
            self.epoll_file,
            epoll::EpollOp::EpollCtlDel,
            fd as RawFd,
            &mut epoll::EpollEvent::new(
                epoll::EpollFlags::EPOLLIN,
                self.signal_fd.as_raw_fd() as u64,
            ),
        );
        if epoll_result.is_err() {
            warn!("Could not unregister fd from epoll");
        }

        let close_result = unistd::close(fd);
        if close_result.is_err() {
            warn!("Could not close fd {}", fd);
        }

        self.fd_dict.remove(&(fd as RawFd));
        debug!("{:?}", self.fd_dict);
    }

    fn handle_child_output(&mut self, fd: RawFd) {
        let mut buffer = [0 as u8; 4096];
        let length = unistd::read(fd, &mut buffer);

        if length.is_ok() {
            let raw_output = String::from_utf8_lossy(&buffer[..length.unwrap()]);
            let output = raw_output.lines();
            let child_name = &self.processes[*self.fd_dict.get(&fd).expect("Invalid fd found")]
                .description
                .name;

            for line in output {
                if !line.is_empty() {
                    info!("Child {}: {}", child_name, line);
                }
            }
        }
    }

    fn kick_off_children(&mut self) {
        while !self.runnable.is_empty() {
            let child_index = self.runnable.pop_back().unwrap();
            let child_result = self.processes[child_index].start();

            if child_result.is_err() {
                error!("Failed to spawn child: {}", child_result.unwrap_err());
                return;
            }
            let child_desc = child_result.unwrap();

            self.pid_dict.insert(child_desc.0, child_index);
            self.register_fd(child_desc.1, child_index);
            self.register_fd(child_desc.2, child_index);
        }
    }
}
