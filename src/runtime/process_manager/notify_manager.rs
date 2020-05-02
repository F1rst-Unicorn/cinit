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

use std::collections::HashMap;

use crate::runtime::process_manager::ProcessManager;
use crate::util::libc_helpers::slice_to_string;

use log::debug;
use log::info;
use log::trace;
use log::warn;

use nix::cmsg_space;
use nix::sys::socket::recvmsg;
use nix::sys::socket::ControlMessageOwned::ScmCredentials;
use nix::sys::socket::MsgFlags;
use nix::sys::socket::UnixCredentials;
use nix::sys::uio::IoVec;
use nix::unistd::Pid;

impl ProcessManager {
    pub fn read_notification(&mut self) {
        if let Err(e) = self.read_notification_internally() {
            warn!("Failed to receive notification: {:#?}", e);
        }
    }

    fn read_notification_internally(&mut self) -> Result<(), nix::Error> {
        let (state, peer) = self.read_socket()?;
        self.process(&state, &peer);
        Ok(())
    }

    fn read_socket(&mut self) -> Result<(String, UnixCredentials), nix::Error> {
        let mut buffer: [u8; 4096] = [0; 4096];
        let mut control = cmsg_space!(UnixCredentials);
        let result = recvmsg(
            self.notify_fd,
            &[IoVec::from_mut_slice(&mut buffer)],
            Some(&mut control),
            MsgFlags::empty(),
        )?;
        let message = slice_to_string(&buffer);
        let peer;
        for m in result.cmsgs() {
            if let ScmCredentials(credentials) = m {
                peer = credentials;
                debug!(
                    "Received notification '{}' len {} from {}",
                    message,
                    message.chars().count(),
                    peer.pid()
                );
                return Ok((message, peer));
            }
        }
        // should not happen as we request so_passcred when opening the socket
        Err(nix::Error::Sys(nix::errno::Errno::EBADMSG))
    }

    fn process(&mut self, state: &str, peer: &UnixCredentials) {
        let variables = ProcessManager::parse(state);
        for (key, value) in variables {
            self.process_variables(&key, &value, peer);
        }
    }

    fn process_variables(&mut self, key: &str, value: &str, peer: &UnixCredentials) {
        match key {
            "READY" => {
                if value != "1" {
                    warn!(
                        "Expected READY=1 but value was '{}' len {}",
                        value,
                        value.chars().count()
                    );
                    return;
                }

                self.handle_started_child(peer)
            }
            "STATUS" => {
                self.handle_published_status(value, peer);
            }
            _ => {
                info!("notify manager ignores variable '{}' = '{}'", key, value);
            }
        };
    }

    fn handle_started_child(&mut self, peer: &UnixCredentials) {
        let pid = Pid::from_raw(peer.pid());
        let process_id_result = self.process_map.process_id_for_pid(pid);
        match process_id_result {
            Some(process_id) => {
                let process = self
                    .process_map
                    .process_for_pid(pid)
                    .expect("process not found although id was found");
                info!("Child {} has started successfully", process.name);
                trace!("Child {} has started successfully", process.name);
                self.dependency_manager.notify_process_finished(process_id);
            }
            None => {
                warn!("Got notification from unknown pid {}", peer.pid());
            }
        }
    }

    fn handle_published_status(&mut self, value: &str, peer: &UnixCredentials) {
        let pid = Pid::from_raw(peer.pid());
        let process_result = self.process_map.process_for_pid(pid);
        match process_result {
            Some(process) => {
                info!("Child {}: {}", process.name, value);
                trace!("Child {}: {}", process.name, value);
                process.status = value.to_string();
            }
            None => {
                warn!("Got notification from unknown pid {}", peer.pid());
            }
        }
    }

    fn parse(state: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for line in state.lines() {
            let mut split = line.splitn(2, '=');
            let key = split.next().expect("At least one split has to exist");
            if let Some(value) = split.next() {
                result.insert(key.to_string(), value.to_string());
            } else {
                warn!("notify_manager failed to parse status line '{}'", line);
            }
        }
        result
    }
}
