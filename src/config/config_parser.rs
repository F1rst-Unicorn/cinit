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
use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::process::exit;
use std::result::Result;

use log::{debug, error, trace, warn};

use crate::config::Config;
use crate::config::ProcessConfig;

const EXIT_CODE: i32 = 1;

pub fn parse_config(path: &str) -> Config {
    let raw_config = read_config(path);
    debug!(
        "Complete configuration:\n{}",
        raw_config
            .iter()
            .flat_map(|s| s.chars())
            .collect::<String>()
    );
    let config = parse_raw_config(&raw_config);

    merge_dropins(config)
}

pub fn read_config(path: &str) -> Vec<String> {
    let metadata_result = fs::metadata(path);

    if let Err(err) = metadata_result {
        error!("Failed to read metadata of {}: {}", path, err);
        exit(EXIT_CODE);
    }

    let mut result: Vec<String>;
    let metadata = metadata_result.unwrap();

    if metadata.file_type().is_dir() {
        let content = fs::read_dir(path);
        if let Err(err) = content {
            error!("Failed to get directory content of {}: {}", path, err);
            exit(EXIT_CODE);
        }

        result = Vec::new();

        for entry in content.unwrap() {
            if let Err(err) = entry {
                error!("Failed to read {}: {}", path, err);
                exit(EXIT_CODE);
            }
            let entry_path = entry.unwrap().path();
            let entry_path_string = entry_path.to_str().unwrap();
            let content = read_config(entry_path_string);

            result.extend(content);
        }
    } else if metadata.file_type().is_file() {
        match read_file(path) {
            Err(error) => {
                error!("Failed to read file {}: {}", path, error);
                exit(EXIT_CODE);
            }
            Ok(content) => {
                result = vec![content];
            }
        }
    } else {
        warn!("Ignoring file {}", path);
        result = Vec::new();
    }

    result
}

fn parse_raw_config(raw_config: &[String]) -> Config {
    let parse_result = raw_config.iter().map(|s| serde_yaml::from_str(s));

    let parse_errors: Vec<serde_yaml::Result<Config>> =
        parse_result.clone().filter(Result::is_err).collect();

    if !parse_errors.is_empty() {
        error!("Could not parse config: ");
        for error in parse_errors {
            error!("{:#?}", error.unwrap_err());
        }
        trace!("Error in configuration file");
        exit(EXIT_CODE);
    } else {
        parse_result
            .map(Result::unwrap)
            .fold(Config::new(), Config::merge)
    }
}

fn merge_dropins(config: Config) -> Config {
    let mut dict: HashMap<String, ProcessConfig> = HashMap::new();

    for process_config in config.programs {
        match dict.remove(&process_config.name) {
            Some(process) => {
                let merged = process.merge(process_config);

                if let Err(e) = merged {
                    error!("{}", e);
                    trace!("{}", e);
                    exit(EXIT_CODE);
                }

                let merged = merged.unwrap();
                dict.insert(merged.name.to_owned(), merged);
            }
            None => {
                dict.insert(process_config.name.to_owned(), process_config);
            }
        };
    }
    let processes = dict.drain().map(|(_, v)| v).collect();
    Config {
        programs: processes,
    }
}

pub fn read_file(file_path: &str) -> Result<String, io::Error> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::super::ProcessType;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn parse_single_program() {
        let mut expected_env = Vec::new();
        let mut entry = HashMap::new();
        entry.insert("key".to_owned(), Some("value".to_owned()));
        expected_env.push(entry);
        let mut entry = HashMap::new();
        entry.insert("empty_key".to_owned(), None);
        expected_env.push(entry);

        let output = parse_raw_config(&vec![FULL_CONFIG.to_owned()]);

        assert_eq!(1, output.programs.len());

        let program = &output.programs[0];
        assert_eq!("test", program.name);
        assert_eq!(Some("/some/path".to_owned()), program.path);
        assert_eq!(Vec::new() as Vec<String>, program.args);
        assert_eq!(Some("/hello/path".to_owned()), program.workdir);
        assert_eq!(ProcessType::Oneshot, program.process_type);
        assert_eq!(Some(3), program.uid);
        assert_eq!(Some(1), program.gid);
        assert_eq!(Some("root".to_owned()), program.user);
        assert_eq!(Some("group".to_owned()), program.group);
        assert_eq!(vec!["ever"], program.before);
        assert_eq!(vec!["after"], program.after);
        assert_eq!(false, program.emulate_pty);
        assert_eq!(vec!["some"], program.capabilities);
        assert_eq!(expected_env, program.env);
    }

    #[test]
    fn parse_omitting_all_optional_values() {
        let output = parse_raw_config(&vec![MINIMAL_CONFIG.to_owned()]);

        assert_eq!(1, output.programs.len());

        let program = &output.programs[0];
        assert_eq!("test", program.name);
        assert_eq!(Some("/path".to_owned()), program.path);
        assert_eq!(Vec::new() as Vec<String>, program.args);
        assert_eq!(None, program.workdir);
        assert_eq!(ProcessType::Oneshot, program.process_type);
        assert_eq!(None, program.uid);
        assert_eq!(None, program.gid);
        assert_eq!(None, program.user);
        assert_eq!(None, program.group);
        assert_eq!(Vec::new() as Vec<String>, program.before);
        assert_eq!(Vec::new() as Vec<String>, program.after);
        assert_eq!(false, program.emulate_pty);
        assert_eq!(Vec::new() as Vec<String>, program.capabilities);
        assert_eq!(
            Vec::new() as Vec<HashMap<String, Option<String>>>,
            program.env
        );
    }

    #[test]
    fn parse_cronjob() {
        let output = parse_raw_config(&vec![CRONJOB_CONFIG.to_owned()]);

        assert_eq!(1, output.programs.len());

        let program = &output.programs[0];
        assert_eq!("test", program.name);
        assert_eq!(Some("/path".to_owned()), program.path);
        assert_eq!(
            ProcessType::CronJob {
                timer: "1 2 3 4 5".to_string()
            },
            program.process_type
        );
    }

    const MINIMAL_CONFIG: &str = "\
programs:
  - name: test
    path: /path
";

    const CRONJOB_CONFIG: &str = "\
programs:
  - name: test
    path: /path
    type:
      cronjob:
        timer: 1 2 3 4 5
";
    const FULL_CONFIG: &str = "\
programs:
  - name: test
    path: /some/path
    args: []
    workdir: /hello/path
    type: oneshot
    uid: 3
    gid: 1
    user: root
    group: group
    before:
      - ever
    after:
      - after
    emulate_pty: false
    capabilities:
      - some
    env:
      - key: value
      - empty_key:
";
}
