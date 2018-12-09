use std::collections::HashMap;
use std::ffi::CString;
use std::fmt::Display;
use std::fmt::Error as FmtError;
use std::fmt::Formatter;
use std::path::PathBuf;

use config::{ProcessConfig, ProcessType};
use runtime::process::{Process, ProcessState};
use util::libc_helpers;

use nix::unistd::Gid;
use nix::unistd::Pid;
use nix::unistd::Uid;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    CronjobDependency,
    UserGroupInvalid,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        let message = match self {
            Error::CronjobDependency => "Cronjobs may not have dependencies",
            Error::UserGroupInvalid => "User/Group config invalid",
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

        let uid = Uid::from_raw(map_uid(config.uid, &config.user)?);
        let gid = Gid::from_raw(map_gid(config.gid, &config.group)?);

        let env = convert_env(&config.env, uid);

        let mut result = Process {
            name: config.name.to_owned(),
            path: config.path.to_owned(),
            args: Vec::new(),
            workdir: PathBuf::from(match &config.workdir {
                None => ".",
                Some(path) => path,
            }),
            uid,
            gid,
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
                .map(|x| CString::new(x.clone()).expect("Could not unwrap arg"))
                .collect(),
        );

        Ok(result)
    }
}

fn sanitise_env(env: &mut HashMap<String, String>, uid: Uid) {
    let homedir = libc_helpers::uid_to_homedir(uid.as_raw()).expect("Could not transform homedir");
    let username = libc_helpers::uid_to_user(uid.as_raw()).expect("Could not transform user name");

    env.insert("HOME".to_string(), homedir.clone());
    env.insert("PWD".to_string(), homedir.clone());
    env.insert("USER".to_string(), username.clone());
    env.insert("LOGNAME".to_string(), username.clone());
    env.insert("SHELL".to_string(), "/bin/sh".to_string());
}

fn map_uid(id: Option<u32>, name: &Option<String>) -> Result<u32, Error> {
    let mapped = map_unix_name(id, name, &libc_helpers::user_to_uid);
    if let Ok(id) = mapped {
        if libc_helpers::is_uid_valid(id) {
            Ok(id)
        } else {
            Err(Error::UserGroupInvalid)
        }
    } else {
        mapped
    }
}

fn map_gid(id: Option<u32>, name: &Option<String>) -> Result<u32, Error> {
    let mapped = map_unix_name(id, name, &libc_helpers::group_to_gid);
    if let Ok(id) = mapped {
        if libc_helpers::is_gid_valid(id) {
            Ok(id)
        } else {
            Err(Error::UserGroupInvalid)
        }
    } else {
        mapped
    }
}

/// Can be used to get either user id or group id
fn map_unix_name<T>(id: Option<u32>, name: &Option<String>, mapper: &T) -> Result<u32, Error>
where
    T: Fn(&str) -> nix::Result<u32>,
{
    if id.is_some() && name.is_some() {
        Err(Error::UserGroupInvalid)
    } else if id.is_some() && name.is_none() {
        Ok(id.unwrap())
    } else if id.is_none() && name.is_some() {
        let mapped = mapper(name.as_ref().unwrap());
        match mapped {
            Ok(id) => Ok(id),
            Err(_) => Err(Error::UserGroupInvalid),
        }
    } else {
        Ok(0)
    }
}

fn convert_env(env: &[HashMap<String, Option<String>>], uid: Uid) -> HashMap<String, String> {
    let mut result = get_default_env();
    sanitise_env(&mut result, uid);
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
                result.insert(key.to_string(), String::from(""));
            }
            Ok(real_value) => {
                result.insert(key.to_string(), real_value);
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
                    let rendered_value =
                        render_template(&result, raw_value).unwrap_or_else(|_| raw_value.to_string());
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
    tera.render(name, &context).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_user_id_gives_error() {
        let result = map_uid(Some(1001), &None);

        assert!(result.is_err());
        assert_eq!(Error::UserGroupInvalid, result.unwrap_err())
    }

    #[test]
    fn invalid_group_id_gives_error() {
        let result = map_gid(Some(1001), &None);

        assert!(result.is_err());
        assert_eq!(Error::UserGroupInvalid, result.unwrap_err())
    }

    #[test]
    fn no_user_config_gives_root() {
        let result = map_unix_name(None, &None, &libc_helpers::group_to_gid);

        assert!(result.is_ok());
        assert_eq!(0, result.unwrap());
    }

    #[test]
    fn both_user_config_gives_error() {
        let result = map_unix_name(
            Some(1000),
            &Some("builder".to_string()),
            &libc_helpers::user_to_uid,
        );

        assert!(result.is_err());
        assert_eq!(Error::UserGroupInvalid, result.unwrap_err());
    }

    #[test]
    fn unknown_user_gives_error() {
        let result = map_unix_name(
            None,
            &Some("unknownuser".to_string()),
            &libc_helpers::user_to_uid,
        );

        assert!(result.is_err());
        assert_eq!(Error::UserGroupInvalid, result.unwrap_err());
    }
}
