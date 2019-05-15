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

pub mod config_parser;

use std::collections::HashMap;

use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum ProcessType {
    #[serde(rename = "oneshot")]
    Oneshot,

    #[serde(rename = "cronjob")]
    CronJob { timer: String },
}

fn default_process_type() -> ProcessType {
    ProcessType::Oneshot
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessConfig {
    pub name: String,

    pub path: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default)]
    pub workdir: Option<String>,

    #[serde(rename = "type")]
    #[serde(default = "default_process_type")]
    pub process_type: ProcessType,

    pub uid: Option<u32>,

    pub gid: Option<u32>,

    pub user: Option<String>,

    pub group: Option<String>,

    #[serde(default)]
    pub before: Vec<String>,

    #[serde(default)]
    pub after: Vec<String>,

    #[serde(rename = "pty")]
    #[serde(default)]
    pub emulate_pty: bool,

    #[serde(default)]
    pub capabilities: Vec<String>,

    #[serde(default)]
    pub env: Vec<HashMap<String, Option<String>>>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub programs: Vec<ProcessConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            programs: Vec::new(),
        }
    }
}

impl Config {
    pub fn new() -> Config {
        Default::default()
    }

    pub fn merge(mut self, mut other: Self) -> Self {
        self.programs.append(&mut other.programs);
        self
    }
}
