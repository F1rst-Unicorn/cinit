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
use std::convert;
use std::ffi::CString;
use std::fmt::Display;
use std::fmt::Error as FmtError;
use std::fmt::Formatter;
use std::path::PathBuf;

use crate::config::{ProcessConfig, ProcessType};
use crate::runtime::process::{Process, ProcessState};

use nix::unistd::Gid;
use nix::unistd::Group;
use nix::unistd::Pid;
use nix::unistd::Uid;
use nix::unistd::User;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    CronjobDependency,
    UserGroupInvalid,
    PathMissing,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        let message = match self {
            Error::CronjobDependency => "Cronjobs may not have dependencies",
            Error::UserGroupInvalid => "User/Group config invalid",
            Error::PathMissing => "Missing 'path' attribute",
        };

        write!(f, "{}", message)
    }
}

impl Process {
    pub fn from(config: &ProcessConfig) -> Result<Process, Error> {
        if let ProcessType::CronJob { .. } = &config.process_type {
            if !config.before.is_empty() || !config.after.is_empty() {
                return Err(Error::CronjobDependency);
            }
        }

        let user = map_uid(config.uid, &config.user)?;
        let group = map_gid(config.gid, &config.group)?;

        let env = convert_env(&config.env, &user);

        if config.path.is_none() {
            return Err(Error::PathMissing);
        }

        let mut result = Process {
            name: config.name.to_owned(),
            path: config.path.to_owned().expect("was checked above"),
            args: Vec::new(),
            workdir: PathBuf::from(match &config.workdir {
                None => ".",
                Some(path) => path,
            }),
            uid: user.uid,
            gid: group.gid,
            emulate_pty: config.emulate_pty,
            capabilities: config.capabilities.to_owned(),
            env: flatten_to_strings(&env),
            state: match config.process_type {
                ProcessType::Oneshot => ProcessState::Blocked,
                ProcessType::CronJob { .. } => ProcessState::Sleeping,
            },
            pid: Pid::from_raw(0),
        };

        result
            .args
            .push(CString::new(result.path.clone()).expect("Could not transform path"));

        result.args.append(
            &mut config
                .args
                .iter()
                .map(|x| render_template(&env, x).unwrap_or_else(|_| x.clone()))
                .map(|x| CString::new(x).expect("Could not unwrap arg"))
                .collect(),
        );

        Ok(result)
    }
}

fn sanitise_env(env: &mut HashMap<String, String>, user: &User) {
    env.insert("HOME".to_string(), user.dir.to_string_lossy().to_string());
    env.insert("PWD".to_string(), user.dir.to_string_lossy().to_string());
    env.insert("USER".to_string(), user.name.clone());
    env.insert("LOGNAME".to_string(), user.name.clone());
    env.insert("SHELL".to_string(), "/bin/sh".to_string());
}

fn map_uid(id: Option<u32>, name: &Option<String>) -> Result<User, Error> {
    map_id(
        id,
        name,
        |v| User::from_uid(Uid::from_raw(v)),
        |v| User::from_name(v),
    )
}

fn map_gid(id: Option<u32>, name: &Option<String>) -> Result<Group, Error> {
    map_id(
        id,
        name,
        |v| Group::from_gid(Gid::from_raw(v)),
        |v| Group::from_name(v),
    )
}

fn map_id<T, F, G>(
    mut id: Option<u32>,
    name: &Option<String>,
    from_id: F,
    from_name: G,
) -> Result<T, Error>
where
    F: Fn(u32) -> Result<Option<T>, nix::Error>,
    G: Fn(&String) -> Result<Option<T>, nix::Error>,
{
    match (id, &name) {
        (None, &None) => id = Some(0),
        (Some(_), &Some(_)) => return Err(Error::UserGroupInvalid),
        _ => {}
    }

    let id = id
        .map(from_id)
        .map(Result::ok)
        .and_then(convert::identity)
        .and_then(convert::identity);
    let name = name
        .as_ref()
        .map(from_name)
        .map(Result::ok)
        .and_then(convert::identity)
        .and_then(convert::identity);
    id.or(name).ok_or(Error::UserGroupInvalid)
}

fn convert_env(env: &[HashMap<String, Option<String>>], user: &User) -> HashMap<String, String> {
    let mut result = get_default_env();
    sanitise_env(&mut result, user);
    copy_from_config(env, result)
}

fn get_default_env() -> HashMap<String, String> {
    let mut result: HashMap<String, String> = HashMap::new();
    let default_env = [
        "HOME", "LANG", "LANGUAGE", "LOGNAME", "PATH", "PWD", "SHELL", "TERM", "USER",
    ];
    for key in default_env.iter() {
        match std::env::var(key) {
            Err(_) => {
                result.insert((*key).to_string(), String::from(""));
            }
            Ok(real_value) => {
                result.insert((*key).to_string(), real_value);
            }
        }
    }
    result
}

fn copy_from_config(
    env: &[HashMap<String, Option<String>>],
    mut result: HashMap<String, String>,
) -> HashMap<String, String> {
    for entry in env {
        for (key, value) in entry {
            match value {
                None => match std::env::var(key) {
                    Err(_) => {}
                    Ok(real_value) => {
                        result.insert(key.to_string(), real_value);
                    }
                },
                Some(raw_value) => {
                    let rendered_value = render_template(&result, raw_value)
                        .unwrap_or_else(|_| raw_value.to_string());
                    result.insert(key.to_string(), rendered_value);
                }
            }
        }
    }
    result
}

fn flatten_to_strings(result: &HashMap<String, String>) -> Vec<CString> {
    let mut ret: Vec<CString> = Vec::new();
    for (key, value) in result.iter() {
        let entry: String = key.to_owned() + "=" + value;
        ret.push(CString::new(entry).expect("Could not build env var"));
    }
    ret
}

fn render_template(context: &HashMap<String, String>, raw_value: &str) -> Result<String, ()> {
    let mut tera: tera::Tera = Default::default();
    let mut internal_context = tera::Context::new();
    let name = "name";
    tera.add_raw_template(name, raw_value).map_err(|_| ())?;
    for (key, value) in context {
        internal_context.insert(key, value);
    }
    tera.render(name, &internal_context).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_user_id_gives_error() {
        let result = map_uid(Some(1410), &None);

        assert!(result.is_err());
        assert_eq!(Error::UserGroupInvalid, result.unwrap_err())
    }

    #[test]
    fn invalid_group_id_gives_error() {
        let result = map_gid(Some(1410), &None);

        assert!(result.is_err());
        assert_eq!(Error::UserGroupInvalid, result.unwrap_err())
    }

    #[test]
    fn no_user_config_gives_root() {
        let result = map_uid(None, &None);

        assert!(result.is_ok());
        assert!(result.unwrap().uid.is_root());
    }

    #[test]
    fn both_user_config_gives_error() {
        let result = map_uid(Some(1000), &Some("builder".to_string()));

        assert!(result.is_err());
        assert_eq!(Error::UserGroupInvalid, result.unwrap_err());
    }

    #[test]
    fn unknown_user_gives_error() {
        let result = map_uid(None, &Some("unknownuser".to_string()));

        assert!(result.is_err());
        assert_eq!(Error::UserGroupInvalid, result.unwrap_err());
    }
}
