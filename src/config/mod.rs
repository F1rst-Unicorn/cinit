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

//! Read the configuration for the analysis phase.

pub mod config_parser;

use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Error as FmtError;
use std::fmt::Formatter;

use serde_derive::Deserialize;

/// Error occurring while merging [ProcessConfigs](ProcessConfig)
#[derive(Debug, PartialEq, Eq)]
pub enum MergeError {
    /// A path is specified more than once for a program
    PathSpecified(String),

    /// A field which is not allowed in a [dropin](ProcessConfig::merge)
    InvalidFieldSpecified(String, String),

    /// The [dropin](ProcessConfig::merge) was specified to be a cronjob
    CronjobSpecified(String),
}

/// Format the error for the user
impl Display for MergeError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            MergeError::PathSpecified(s) => write!(f, "Duplicate program found for name {s}"),
            MergeError::InvalidFieldSpecified(name, field) => write!(
                f,
                "Configuration drop-in for {name} contains duplicate field {field}"
            ),
            MergeError::CronjobSpecified(s) => {
                write!(f, "Configuration drop-in for {s} changes type to cronjob")
            }
        }
    }
}

/// Programmatic pendant for
/// [ProcessTypes](https://j.njsm.de/git/veenj/cinit/src/branch/master/doc/README.md#program-types)
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub enum ProcessType {
    /// Default "simple" process type running once
    #[serde(rename = "oneshot")]
    Oneshot,

    /// Process type running once and notifying cinit of events
    #[serde(rename = "notify")]
    Notify,

    /// Cronjob with timer expression
    ///
    /// The timer contains the cron expression
    #[serde(rename = "cronjob")]
    CronJob { timer: String },
}

fn default_process_type() -> ProcessType {
    ProcessType::Oneshot
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct CronJob {
    pub timer: String,
}

/// (Partial) Configuration of a single process
///
/// This is the programatic pendant of the configuration file
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct ProcessConfig {
    pub name: String,

    pub path: Option<String>,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default)]
    pub workdir: Option<String>,

    #[serde(rename = "type")]
    #[serde(default = "default_process_type")]
    #[serde(with = "serde_yaml::with::singleton_map")]
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

impl ProcessConfig {
    /// Merge two [ProcessConfig]s according to the [documentation
    /// on
    /// merging](https://j.njsm.de/git/veenj/cinit/src/branch/master/doc/README.md#merging-configuration)
    ///
    /// The [ProcessConfig] containing the `path` (which is only allowed in one place)
    /// is considered the primary one, the other one the dropin.
    ///
    /// # Errors
    ///
    /// If the dropin contains values which are only allowed in one of the configurations
    /// an approriate [MergeError] is raised.
    pub fn merge(self, other: Self) -> Result<Self, MergeError> {
        assert_eq!(self.name, other.name);

        let mut primary;
        let mut dropin = if self.path.is_none() {
            primary = other;
            self
        } else {
            primary = self;
            other
        };

        if dropin.path.is_some() {
            return Err(MergeError::PathSpecified(primary.name));
        }

        if dropin.workdir.is_some() && primary.workdir.is_some() {
            return Err(MergeError::InvalidFieldSpecified(
                primary.name,
                "workdir".to_owned(),
            ));
        }
        if dropin.uid.is_some() && primary.uid.is_some() {
            return Err(MergeError::InvalidFieldSpecified(
                primary.name,
                "uid".to_owned(),
            ));
        }
        if dropin.gid.is_some() && primary.gid.is_some() {
            return Err(MergeError::InvalidFieldSpecified(
                primary.name,
                "gid".to_owned(),
            ));
        }
        if dropin.user.is_some() && primary.user.is_some() {
            return Err(MergeError::InvalidFieldSpecified(
                primary.name,
                "user".to_owned(),
            ));
        }
        if dropin.group.is_some() && primary.group.is_some() {
            return Err(MergeError::InvalidFieldSpecified(
                primary.name,
                "group".to_owned(),
            ));
        }

        if let ProcessType::CronJob { .. } = dropin.process_type {
            return Err(MergeError::CronjobSpecified(primary.name));
        }

        primary.env.append(&mut dropin.env);
        primary.before.append(&mut dropin.before);
        primary.after.append(&mut dropin.after);
        primary.capabilities.append(&mut dropin.capabilities);
        primary.args.append(&mut dropin.args);
        primary.emulate_pty |= dropin.emulate_pty;
        primary.workdir = primary.workdir.or(dropin.workdir);
        primary.uid = primary.uid.or(dropin.uid);
        primary.gid = primary.gid.or(dropin.gid);
        primary.user = primary.user.or(dropin.user);
        primary.group = primary.group.or(dropin.group);

        ProcessConfig::prune_duplicates(&mut primary.before);
        ProcessConfig::prune_duplicates(&mut primary.after);
        ProcessConfig::prune_duplicates(&mut primary.capabilities);

        Ok(primary)
    }

    fn prune_duplicates<T: Ord>(list: &mut Vec<T>) {
        list.sort_unstable();
        list.dedup();
    }
}

/// Top-level Configuration
///
/// Consists of all programs that are known
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub programs: Vec<ProcessConfig>,
}

impl Config {
    /// Merge two [Config]s into one
    pub fn merge(mut self, mut other: Self) -> Self {
        self.programs.append(&mut other.programs);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn merging_different_name_causes_panic() {
        let primary = get_default_config("test");
        let dropin = get_default_config("test2");
        let _ = primary.merge(dropin);
    }

    #[test]
    fn merging_with_two_paths_gives_error() {
        let primary = get_default_config("test");
        let mut dropin = get_default_config("test");
        dropin.path = Some("forbidden".to_owned());

        let output = primary.merge(dropin);

        assert_eq!(Err(MergeError::PathSpecified("test".to_owned())), output);
    }

    #[test]
    fn merging_with_workdir_gives_error() {
        let mut primary = get_default_config("test");
        let mut dropin = get_default_config("test");
        primary.workdir = Some("forbidden".to_owned());
        dropin.workdir = Some("forbidden".to_owned());
        dropin.path = None;

        let output = primary.merge(dropin);

        assert_eq!(
            Err(MergeError::InvalidFieldSpecified(
                "test".to_owned(),
                "workdir".to_owned()
            )),
            output
        );
    }

    #[test]
    fn merging_with_uid_gives_error() {
        let mut primary = get_default_config("test");
        let mut dropin = get_default_config("test");
        primary.uid = Some(1);
        dropin.uid = Some(1);
        dropin.path = None;

        let output = primary.merge(dropin);

        assert_eq!(
            Err(MergeError::InvalidFieldSpecified(
                "test".to_owned(),
                "uid".to_owned()
            )),
            output
        );
    }

    #[test]
    fn merging_with_gid_gives_error() {
        let mut primary = get_default_config("test");
        let mut dropin = get_default_config("test");
        primary.gid = Some(1);
        dropin.gid = Some(1);
        dropin.path = None;

        let output = primary.merge(dropin);

        assert_eq!(
            Err(MergeError::InvalidFieldSpecified(
                "test".to_owned(),
                "gid".to_owned()
            )),
            output
        );
    }

    #[test]
    fn merging_with_user_gives_error() {
        let mut primary = get_default_config("test");
        let mut dropin = get_default_config("test");
        primary.user = Some("forbidden".to_owned());
        dropin.user = Some("forbidden".to_owned());
        dropin.path = None;

        let output = primary.merge(dropin);

        assert_eq!(
            Err(MergeError::InvalidFieldSpecified(
                "test".to_owned(),
                "user".to_owned()
            )),
            output
        );
    }

    #[test]
    fn merging_with_group_gives_error() {
        let mut primary = get_default_config("test");
        let mut dropin = get_default_config("test");
        primary.group = Some("forbidden".to_owned());
        dropin.group = Some("forbidden".to_owned());
        dropin.path = None;

        let output = primary.merge(dropin);

        assert_eq!(
            Err(MergeError::InvalidFieldSpecified(
                "test".to_owned(),
                "group".to_owned()
            )),
            output
        );
    }

    #[test]
    fn merging_with_cronjob_gives_error() {
        let primary = get_default_config("test");
        let mut dropin = get_default_config("test");
        dropin.process_type = ProcessType::CronJob {
            timer: "".to_owned(),
        };
        dropin.path = None;

        let output = primary.merge(dropin);

        assert_eq!(Err(MergeError::CronjobSpecified("test".to_owned())), output);
    }

    #[test]
    fn relevant_lists_are_merged_and_pruned() {
        let list = vec!["1".to_owned(), "2".to_owned()];
        let mut primary = get_default_config("test");
        let mut dropin = get_default_config("test");
        dropin.path = None;
        dropin.before = list.clone();
        dropin.after = list.clone();
        dropin.capabilities = list.clone();
        dropin.args = list.clone();
        dropin.env = vec![HashMap::new()];
        primary.before = list.clone();
        primary.after = list.clone();
        primary.capabilities = list.clone();
        primary.args = list.clone();
        primary.env = vec![HashMap::new()];

        let output = primary.merge(dropin);

        assert!(output.is_ok());
        let merged = output.unwrap();
        assert_eq!(2, merged.env.len());
        assert_eq!(
            vec![
                "1".to_owned(),
                "2".to_owned(),
                "1".to_owned(),
                "2".to_owned()
            ],
            merged.args
        );
        assert_eq!(list, merged.before);
        assert_eq!(list, merged.after);
        assert_eq!(list, merged.capabilities);
    }

    #[test]
    fn args_of_primary_are_kept_first() {
        let mut primary = get_default_config("test");
        let mut dropin = get_default_config("test");
        dropin.path = None;
        dropin.args = vec!["1".to_owned()];
        primary.args = vec!["2".to_owned()];

        let output = primary.merge(dropin);

        assert!(output.is_ok());
        let merged = output.unwrap();
        assert_eq!(vec!["2".to_owned(), "1".to_owned()], merged.args);
    }

    #[test]
    fn args_of_primary_are_kept_first_with_reverse_arguments() {
        let mut primary = get_default_config("test");
        let mut dropin = get_default_config("test");
        dropin.path = None;
        dropin.args = vec!["1".to_owned()];
        primary.args = vec!["2".to_owned()];

        let output = dropin.merge(primary);

        assert!(output.is_ok());
        let merged = output.unwrap();
        assert_eq!(vec!["2".to_owned(), "1".to_owned()], merged.args);
    }

    fn get_default_config(name: &str) -> ProcessConfig {
        ProcessConfig {
            name: name.to_owned(),
            path: Some("test".to_owned()),
            args: vec![],
            workdir: None,
            process_type: ProcessType::Oneshot,
            uid: None,
            gid: None,
            user: None,
            group: None,
            before: vec![],
            after: vec![],
            emulate_pty: false,
            capabilities: vec![],
            env: vec![],
        }
    }
}
