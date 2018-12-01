use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;
use std::process::exit;

use logging;
use runtime::dependency_graph;
use runtime::process::ProcessState;
use runtime::process_map::ProcessMap;

use nix::sys::epoll;
use nix::sys::signal;
use nix::sys::signalfd;
use nix::sys::wait;
use nix::unistd;
use nix::unistd::Pid;

const EXIT_CODE: i32 = 3;

#[derive(Debug)]
pub struct ProcessManager {
    pub process_map: ProcessMap,

    pub keep_running: bool,

    pub dependency_manager: dependency_graph::DependencyManager,

    pub epoll_file: RawFd,

    pub signal_fd: signalfd::SignalFd,
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        let raw_signal_fd = self.signal_fd.as_raw_fd().clone();
        self.deregister_fd(raw_signal_fd);

        if let Err(some) = unistd::close(self.epoll_file) {
            warn!("Could not close epoll fd: {}", some);
        }
    }
}

impl ProcessManager {
    pub fn start(&mut self) {
        match self.setup() {
            Err(content) => {
                error!("Failed to register with epoll: {}", content);
                exit(EXIT_CODE);
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
    }

    fn look_for_finished_children(&mut self) {
        let mut wait_args = wait::WaitPidFlag::empty();
        wait_args.insert(wait::WaitPidFlag::WNOHANG);
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
        let child_index = self.process_map.process_id_for_pid(pid);
        let child_crashed: bool;
        {
            let child = &mut self.process_map.process_for_pid(pid);
            child.state = if rc == 0 {
                info!("Child {} exited successfully", child.name);
                trace!("Child {} exited successfully", child.name);
                child_crashed = false;
                ProcessState::Done
            } else {
                error!("Child {} crashed with {}", child.name, rc);
                trace!("Child {} crashed with {}", child.name, rc);
                child_crashed = true;
                ProcessState::Crashed
            }
        }

        if child_crashed {
            self.initiate_shutdown(signal::SIGINT);
        }

        self.process_map.deregister_pid(pid);
        self.dependency_manager.notify_process_finished(child_index);
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
                self.print_child_output(fd);
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

    fn initiate_shutdown(&mut self, signal: signal::Signal) {
        info!("Received termination signal");
        self.keep_running = false;
        self.signal_children(signal);
    }

    fn signal_children(&mut self, signal: signal::Signal) {
        info!("Killing children");
        for child in self.process_map.processes()
            .iter()
            .filter(|s| s.state == ProcessState::Running)
        {
            signal::kill(child.pid, signal).expect("Could not transmit signal to child");
        }
    }

    fn register_fd(&mut self, fd: RawFd) {
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
    }

    fn deregister_fd(&mut self, fd: RawFd) {
        debug!("Deregistering fd {}", fd);
        let epoll_result = epoll::epoll_ctl(
            self.epoll_file,
            epoll::EpollOp::EpollCtlDel,
            fd as RawFd,
            &mut epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, fd as u64),
        );
        if epoll_result.is_err() {
            warn!("Could not unregister fd from epoll");
        }

        let close_result = unistd::close(fd);
        if close_result.is_err() {
            warn!("Could not close fd {}", fd);
        }

        self.process_map.deregister_fd(&fd);
    }

    fn print_child_output(&mut self, fd: RawFd) {
        let mut buffer = [0 as u8; 4096];
        let length = unistd::read(fd, &mut buffer);

        if length.is_ok() {
            let raw_output = String::from_utf8_lossy(&buffer[..length.unwrap()]);
            let output = raw_output.lines();
            let is_stdout = self.process_map.is_stdout(fd);
            let child_name = &self.process_map.process_for_fd(&fd).name;

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

    fn spawn_children(&mut self) {
        while let Some(child_index) = self.dependency_manager.pop_runnable() {
            let child_result = self.process_map[child_index].start();

            if child_result.is_err() {
                error!("Failed to spawn child: {}", child_result.unwrap_err());
                return;
            }
            let child = child_result.unwrap();

            self.process_map.register_pid(child_index, child.0);
            self.process_map.register_stdout(child_index, child.1);
            self.process_map.register_stderr(child_index, child.2);
            self.register_fd(child.1);
            self.register_fd(child.2);
        }
    }
}
