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

//! Additions to [ProcessManager] to report the runtime status

use crate::runtime::process::{ProcessState, ProcessType};
use crate::runtime::process_manager::ProcessManager;
use crate::util::libc_helpers;

use log::warn;

use nix::sys::socket;
use nix::unistd;

use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::FromRawFd;

impl ProcessManager {
    /// Print the runtime state handling potential errors
    pub fn report_status(&mut self) {
        if let Err(e) = self.write_report() {
            warn!("Failed to print report: {:#?}", e);
        }
    }

    /// Open the socket and write a report to it
    fn write_report(&mut self) -> Result<(), nix::Error> {
        let mut file =
            unsafe { std::fs::File::from_raw_fd(socket::accept(self.status_fd.as_raw_fd())?) };

        self.format_report(&mut file)?;

        unistd::close(file.as_raw_fd())?;
        Ok(())
    }

    /// Generate the report and write it to a stream
    fn format_report<W: Write>(&mut self, file: &mut W) -> Result<(), nix::Error> {
        file.write_fmt(format_args!("programs:\n"))
            .map_err(libc_helpers::map_to_errno)?;
        for (id, p) in self.process_map.processes().iter().enumerate() {
            file.write_fmt(format_args!(
                "  - name: '{}'\n    state: '{}'\n",
                p.name, p.state
            ))
            .map_err(libc_helpers::map_to_errno)?;

            if !p.status.is_empty() {
                file.write_fmt(format_args!("    status: {}\n", p.status))
                    .map_err(libc_helpers::map_to_errno)?;
            }

            match p.state {
                ProcessState::Done => {
                    file.write_fmt(format_args!("    exit_code: 0\n"))
                        .map_err(libc_helpers::map_to_errno)?;
                }
                ProcessState::Crashed(rc) => {
                    file.write_fmt(format_args!("    exit_code: {}\n", rc))
                        .map_err(libc_helpers::map_to_errno)?;
                }
                _ => {}
            }

            if self.process_map.process_id_for_pid(p.pid).is_some() {
                file.write_fmt(format_args!("    pid: {}\n", p.pid))
                    .map_err(libc_helpers::map_to_errno)?;
            }

            if p.process_type == ProcessType::Cronjob {
                file.write_fmt(format_args!(
                    "    scheduled_at: '{}'\n",
                    &self.cron.get_next_execution(id).to_rfc3339()
                ))
                .map_err(libc_helpers::map_to_errno)?;
            }
        }
        Ok(())
    }
}
