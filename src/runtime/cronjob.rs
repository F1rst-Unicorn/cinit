use config::config::{ProcessConfig, ProcessType};

use std::collections::BTreeSet;
use std::collections::BTreeMap;
use std::collections::HashMap;

use time::Tm;
use time::Duration;

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
        let mut iter = raw_desc.split_whitespace();
        let result = Ok(TimerDescription {
            minute: parse_element(iter.next(), 0, 59)?,
            hour: parse_element(iter.next(), 0, 23)?,
            day: parse_element(iter.next(), 1, 31)?,
            month: parse_element(iter.next(), 1, 12)?,
            weekday: parse_element(iter.next(), 0, 6)?,
        });

        if let None = iter.next() {
            result
        } else {
            Err("Too many timer specs".to_string())
        }
    }

    pub fn get_next_execution(&self, from_timepoint: Tm) -> Tm {
        from_timepoint
    }

}

fn parse_element(input: Option<&str>, min: i32, max: i32) -> Result<BTreeSet<i32>, String> {
    if min > max {
        return Err("Invalid range given".to_string());
    }

    match input {
        None => { Err("Incomplete timer spec".to_string()) },

        Some(timespec) => {
            let mut result = BTreeSet::new();
            if timespec == "" {
                return Err("Incomplete timer spec".to_string());

            } else if timespec == "*" {
                for i in min..=max {
                    result.insert(i);
                }

            } else {
                let mut intervals = timespec.split(",");

                while let Some(interval) = intervals.next() {
                    let mut values = interval.split("-");
                    let begin = values.next()
                        .ok_or("Invalid timespec")?
                        .parse::<i32>()
                        .map_err(|_| "Invalid number")?;

                    if begin < min || max < begin  {
                        return Err("Invalid range in timer spec".to_string());
                    }

                    if let Some(end_str) = values.next() {
                        let end = end_str.parse::<i32>()
                            .map_err(|_| "Invalid number")?;

                        if end < min || max < end {
                            return Err("Invalid range in timer spec" .to_string());
                        }

                        if end < begin {
                            return Err("Interval end < begin".to_string())
                        }

                        for i in begin..=end {
                            result.insert(i);
                        }
                    } else {
                        result.insert(begin);
                    }
                }
            }

            Ok(result)
        }
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

    pub fn pop_runnable(&mut self, now: Tm) -> Option<usize> {
        None
    }

    pub fn is_cronjob(&self, id: usize) -> bool {
        self.timers.contains_key(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_star() {
        let result = parse_element(Some("*"), 5, 8);

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(4, map.len());
        assert!(map.contains(&5));
        assert!(map.contains(&6));
        assert!(map.contains(&7));
        assert!(map.contains(&8));
    }

    #[test]
    fn parse_single_number() {
        let result = parse_element(Some("4"), 0, 99);

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(1, map.len());
        assert!(map.contains(&4));
    }

    #[test]
    fn parse_interval() {
        let result = parse_element(Some("4-6"), 0, 99);

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(3, map.len());
        assert!(map.contains(&4));
        assert!(map.contains(&5));
        assert!(map.contains(&6));
    }

    #[test]
    fn parse_enum() {
        let result = parse_element(Some("4,8,16,32,64"), 0, 99);

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(5, map.len());
        assert!(map.contains(&4));
        assert!(map.contains(&8));
        assert!(map.contains(&16));
        assert!(map.contains(&32));
        assert!(map.contains(&64));
    }

    #[test]
    fn parse_two_intervals() {
        let result = parse_element(Some("4-6,44-46"), 0, 99);

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(6, map.len());
        assert!(map.contains(&4));
        assert!(map.contains(&5));
        assert!(map.contains(&4));
        assert!(map.contains(&44));
        assert!(map.contains(&45));
        assert!(map.contains(&46));
    }

    #[test]
    fn parse_complex() {
        let result = parse_element(Some("4,77,44-46,3,95-99"), 0, 99);

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(11, map.len());
        assert!(map.contains(&3));
        assert!(map.contains(&4));
        assert!(map.contains(&44));
        assert!(map.contains(&45));
        assert!(map.contains(&46));
        assert!(map.contains(&77));
        assert!(map.contains(&95));
        assert!(map.contains(&96));
        assert!(map.contains(&97));
        assert!(map.contains(&98));
        assert!(map.contains(&99));
    }

    #[test]
    fn parse_none() {
        let result = parse_element(None, 0, 99);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Incomplete timer spec", message);
    }

    #[test]
    fn parse_empty() {
        let result = parse_element(Some(""), 0, 99);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Incomplete timer spec", message);
    }

    #[test]
    fn parse_out_of_range() {
        let result = parse_element(Some("4"), 0, 3);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Invalid range in timer spec", message);
    }

    #[test]
    fn parse_invalid_interval() {
        let result = parse_element(Some("4-3"), 0, 99);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Interval end < begin", message);
    }

    #[test]
    fn parse_interval_out_of_range_right_overlap() {
        let result = parse_element(Some("2-4"), 0, 3);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Invalid range in timer spec", message);
    }

    #[test]
    fn parse_interval_out_of_range_complete_right() {
        let result = parse_element(Some("4-5"), 0, 3);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Invalid range in timer spec", message);
    }

    #[test]
    fn parse_interval_out_of_range_left_overlap() {
        let result = parse_element(Some("4-6"), 5, 7);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Invalid range in timer spec", message);
    }

    #[test]
    fn parse_interval_out_of_range_complete_left() {
        let result = parse_element(Some("3-4"), 5, 7);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Invalid range in timer spec", message);
    }

    #[test]
    fn parse_invalid_range() {
        let result = parse_element(Some("*"), 9, 7);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Invalid range given", message);
    }

    #[test]
    fn parse_invalid_digit() {
        let result = parse_element(Some("a"), 0, 99);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Invalid number", message);
    }

    #[test]
    fn parse_invalid_digit_in_interval() {
        let result = parse_element(Some("5-a"), 0, 99);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Invalid number", message);
    }

    #[test]
    fn parse_invalid_digit_in_enum() {
        let result = parse_element(Some("5,a"), 0, 99);

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Invalid number", message);
    }

    #[test]
    fn parse_entire_cron_expression() {
        let result = TimerDescription::parse("1 2 3 4 5");

        assert!(result.is_ok());
        let timer = result.unwrap();
        assert_eq!(1, timer.minute.len());
        assert!(timer.minute.contains(&1));
        assert_eq!(1, timer.hour.len());
        assert!(timer.hour.contains(&2));
        assert_eq!(1, timer.day.len());
        assert!(timer.day.contains(&3));
        assert_eq!(1, timer.month.len());
        assert!(timer.month.contains(&4));
        assert_eq!(1, timer.weekday.len());
        assert!(timer.weekday.contains(&5));
    }

    #[test]
    fn parse_entire_cron_expression_with_whitespace() {
        let result = TimerDescription::parse("1 \n2 \t3   4   5");

        assert!(result.is_ok());
        let timer = result.unwrap();
        assert_eq!(1, timer.minute.len());
        assert!(timer.minute.contains(&1));
        assert_eq!(1, timer.hour.len());
        assert!(timer.hour.contains(&2));
        assert_eq!(1, timer.day.len());
        assert!(timer.day.contains(&3));
        assert_eq!(1, timer.month.len());
        assert!(timer.month.contains(&4));
        assert_eq!(1, timer.weekday.len());
        assert!(timer.weekday.contains(&5));
    }

    #[test]
    fn parse_too_short_expr() {
        let result = TimerDescription::parse("1 2 3 4");

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Incomplete timer spec", message);
    }

    #[test]
    fn parse_too_long_expr() {
        let result = TimerDescription::parse("1 2 3 4 5 6");

        assert!(result.is_err());
        let message = result.unwrap_err();
        assert_eq!("Too many timer specs", message);
    }

}