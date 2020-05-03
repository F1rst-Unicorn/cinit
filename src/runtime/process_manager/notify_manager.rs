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
use std::collections::HashSet;

use crate::runtime::process::ProcessType;
use crate::runtime::process_manager::ProcessManager;
use crate::util::libc_helpers::slice_to_string;

use log::debug;
use log::info;
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
                debug!("Received notification '{}' from {}", message, peer.pid());
                return Ok((message, peer));
            }
        }
        // should not happen as we request so_passcred when opening the socket
        Err(nix::Error::Sys(nix::errno::Errno::EBADMSG))
    }

    fn process(&mut self, state: &str, peer: &UnixCredentials) {
        let pid = Pid::from_raw(peer.pid());
        let process_id_result = self.process_map.process_id_for_pid(pid);
        if let Some(process_id) = process_id_result {
            let process = self
                .process_map
                .process_for_pid(pid)
                .expect("process id not found although process was found");

            if process.process_type != ProcessType::Notify {
                warn!("cinit only accepts notifications from processes of type notify");
                warn!("{} is not allowed to send notifications", process.name);
                return;
            }

            let variables = ProcessManager::parse(state);
            for (key, value) in &variables {
                process.handle_notification(key, value);
            }
            for (key, value) in &variables {
                self.handle_notification(process_id, key, value);
            }
        } else {
            warn!("Got notification from unknown pid {}", peer.pid());
        }
    }

    fn handle_notification(&mut self, process_id: usize, key: &str, value: &str) {
        if key == "READY" {
            if value != "1" {
                warn!("Expected READY=1 but value was '{}'", value);
                return;
            }

            self.dependency_manager.notify_process_finished(process_id);
        }
    }

    fn parse(state: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        let mut allowed_keys = HashSet::new();
        allowed_keys.insert("READY");
        allowed_keys.insert("STOPPING");
        allowed_keys.insert("STATUS");

        for line in state.lines() {
            let mut split = line.splitn(2, '=');
            let key = split.next().expect("At least one split has to exist");
            if let Some(value) = split.next() {
                if !allowed_keys.contains(key) {
                    info!("notify manager ignores variable '{}' = '{}'", key, value);
                    continue;
                }
                result.insert(key.to_string(), value.to_string());
            } else {
                warn!("notify_manager failed to parse status line '{}'", line);
            }
        }
        result
    }
}
