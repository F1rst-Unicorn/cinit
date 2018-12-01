use config::config::{ProcessConfig, ProcessType};

use std::collections::BTreeSet;
use std::collections::BTreeMap;
use std::collections::HashMap;

use time::Tm;

#[derive(Debug)]
pub struct TimerDescription {
    minute: BTreeSet<i32>,

    hour: BTreeSet<i32>,

    day: BTreeSet<i32>,

    month: BTreeSet<i32>,

    weekday: BTreeSet<i32>,
}

impl TimerDescription {

    pub fn parse(raw_desc: &str) -> Result<TimerDescription, String> {
        Err("Not implemented".to_string())
    }

    pub fn get_next_execution(&self, from_timepoint: Tm) -> Tm {
        from_timepoint
    }
}

#[derive(Debug)]
pub enum Error {
    TimeParseError(String, usize),
}

#[derive(Debug)]
pub struct Cron {
    timers: HashMap<usize, TimerDescription>,

    timer: BTreeMap<Tm, usize>,
}

impl Cron {

    pub fn with_jobs(config: &Vec<(usize, ProcessConfig)>) -> Result<Cron, Error> {
        let mut result = Cron {
            timers: HashMap::new(),
            timer: BTreeMap::new(),
        };

        for (id, program_config) in config {

            let raw_desc = match &program_config.process_type {
                ProcessType::CronJob { timer: desc } => desc,
                _ => panic!("Got invalid process type"),
            };

            let time_desc = TimerDescription::parse(&raw_desc)
                .map_err(|s| Error::TimeParseError(s, *id))?;
            result.timer.insert(time_desc.get_next_execution(time::now()), *id);
            result.timers.insert(*id, time_desc);
        }

        Ok(result)
    }

    pub fn get_eligible_jobs(&mut self, now: Tm) -> Vec<usize> {
        Vec::new()
    }
}