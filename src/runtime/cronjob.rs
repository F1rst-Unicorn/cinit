use crate::config::{ProcessConfig, ProcessType};

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;

use time::Duration;
use time::Tm;

use log::debug;

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

            // account for zero-basing in struct Tm
            month: parse_element(iter.next(), 1, 12)?
                .iter()
                .map(|i| i - 1)
                .collect(),
            weekday: parse_element(iter.next(), 0, 6)?,
        });

        if iter.next().is_none() {
            result
        } else {
            Err("Too many timer specs".to_string())
        }
    }

    pub fn get_next_execution(&self, from_timepoint: Tm) -> Tm {
        let mut result = from_timepoint;
        let mut carry = 0;

        result.tm_min = match self.minute.range((from_timepoint.tm_min + 1)..).next() {
            Some(&min) => min,
            None => {
                carry = 1;
                *self.minute.iter().next().unwrap()
            }
        };

        result.tm_hour = match self.hour.range((from_timepoint.tm_hour + carry)..).next() {
            Some(&h) => {
                carry = 0;
                h
            }
            None => {
                carry = 1;
                *self.hour.iter().next().unwrap()
            }
        };

        let next_weekday = match self
            .weekday
            .range((from_timepoint.tm_wday + carry)..)
            .next()
        {
            Some(&day) => day,
            None => *self.weekday.iter().next().unwrap(),
        };

        let next_day = match self.day.range((from_timepoint.tm_mday + carry)..).next() {
            Some(&day) => {
                carry = 0;
                day
            }
            None => {
                carry = 1;
                *self.day.iter().next().unwrap()
            }
        };

        let next_month = match self.month.range((from_timepoint.tm_mon + carry)..).next() {
            Some(&month) => {
                carry = 0;
                month
            }
            None => {
                carry = 1;
                *self.month.iter().next().unwrap()
            }
        };

        let weekday_relevant = self.weekday.len() != 7;
        let date_relevant = self.day.len() != 31 || self.month.len() != 12;

        let week_duration = Duration::days(i64::from(if next_weekday < result.tm_wday {
            7 - (result.tm_wday - next_weekday)
        } else {
            next_weekday - result.tm_wday
        }));

        let mut date_duration = Duration::days(i64::from(carry) * 365 as i64);
        if date_relevant {
            // only compute this if really needed
            let mut tmp = result + date_duration;
            while tmp.tm_mday != next_day || tmp.tm_mon != next_month {
                date_duration = date_duration + Duration::days(1);
                tmp = result + date_duration;
            }
        }

        if weekday_relevant && date_relevant {
            (result + std::cmp::min(week_duration, date_duration)).to_local()
        } else if !weekday_relevant && date_relevant {
            (result + date_duration).to_local()
        } else {
            // For only weekday_relevant this is obviously the result
            // If none of the flags are set, any day works which is expressed
            // already by the week_duration
            (result + week_duration).to_local()
        }
    }
}

fn parse_element(input: Option<&str>, min: i32, max: i32) -> Result<BTreeSet<i32>, String> {
    if min > max {
        return Err("Invalid range given".to_string());
    }

    match input {
        None => Err("Incomplete timer spec".to_string()),

        Some(timespec) => {
            let mut result = BTreeSet::new();
            if timespec == "" {
                return Err("Incomplete timer spec".to_string());
            } else if timespec == "*" {
                for i in min..=max {
                    result.insert(i);
                }
            } else {
                let intervals = timespec.split(',');

                for interval in intervals {
                    let mut values = interval.split('-');
                    let begin = values
                        .next()
                        .ok_or("Invalid timespec")?
                        .parse::<i32>()
                        .map_err(|_| "Invalid number")?;

                    if begin < min || max < begin {
                        return Err("Invalid range in timer spec".to_string());
                    }

                    if let Some(end_str) = values.next() {
                        let mut step_split = end_str.split('/');

                        let end = step_split
                            .next()
                            .ok_or("Invalid timespec")?
                            .parse::<i32>()
                            .map_err(|_| "Invalid number")?;

                        if end < min || max < end {
                            return Err("Invalid range in timer spec".to_string());
                        }

                        let step = if let Some(step) = step_split.next() {
                            step.parse::<i32>().map_err(|_| "Invalid number")?
                        } else {
                            1
                        };

                        if end < begin {
                            return Err("Interval end < begin".to_string());
                        }

                        for i in begin..=end {
                            if i % step == begin % step {
                                result.insert(i);
                            }
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
    pub fn with_jobs(config: &[(usize, ProcessConfig)]) -> Result<Cron, Error> {
        let mut result = Cron {
            timers: HashMap::new(),
            timer: BTreeMap::new(),
        };

        for (id, program_config) in config {
            let raw_desc = match &program_config.process_type {
                ProcessType::CronJob { timer: desc } => desc,
                _ => panic!("Got invalid process type"),
            };

            let time_desc =
                TimerDescription::parse(&raw_desc).map_err(|s| Error::TimeParseError(s, *id))?;
            let next_execution = time_desc.get_next_execution(time::now());
            debug!(
                "Scheduled execution of '{}' at {}",
                program_config.name,
                time::strftime("%FT%T", &next_execution).unwrap()
            );
            result.timer.insert(next_execution, *id);
            result.timers.insert(*id, time_desc);
        }

        Ok(result)
    }

    pub fn pop_runnable(&mut self, now: Tm) -> Option<usize> {
        let next_job = self.timer.iter().next().map(|t| (*t.0, *t.1));

        if let Some((next_exec_time, process_id)) = next_job {
            if next_exec_time <= now {
                self.timer.remove(&next_exec_time);
                let next_execution = self.timers[&process_id].get_next_execution(now);
                debug!(
                    "Scheduled next execution at {}",
                    time::strftime("%FT%T", &next_execution).unwrap()
                );
                self.timer.insert(next_execution, process_id);
                Some(process_id)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn is_cronjob(&self, id: usize) -> bool {
        self.timers.contains_key(&id)
    }

    pub fn get_next_execution(&self, id: usize) -> Tm {
        for (time, item_id) in &self.timer {
            if id == *item_id {
                return *time;
            }
        }
        panic!("Queried cron manager with invalid id");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::*;

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
    fn parse_interval_with_stepping() {
        let result = parse_element(Some("1-15/3"), 0, 99);

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(5, map.len());
        assert!(map.contains(&1));
        assert!(map.contains(&4));
        assert!(map.contains(&7));
        assert!(map.contains(&10));
        assert!(map.contains(&13));
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
    fn parse_invalid_interval_with_stepping() {
        let result = parse_element(Some("1-15/x"), 0, 99);

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
        assert!(timer.month.contains(&3));
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
        assert!(timer.month.contains(&3));
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

    #[test]
    fn advance_by_one_minute() {
        let uut = TimerDescription::parse("* * * * *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::minutes(1)).to_local(), result);
    }

    #[test]
    fn advance_by_two_minutes() {
        let uut = TimerDescription::parse("32 * * * *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::minutes(2)).to_local(), result);
    }

    #[test]
    fn advance_wrap_around_minutes() {
        let uut = TimerDescription::parse("29 * * * *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::minutes(59)).to_local(), result);
    }

    #[test]
    fn advance_by_one_hour() {
        let uut = TimerDescription::parse("30 * * * *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::hours(1)).to_local(), result);
    }

    #[test]
    fn advance_by_two_hours() {
        let uut = TimerDescription::parse("30 14 * * *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::hours(2)).to_local(), result);
    }

    #[test]
    fn advance_wrap_around_hours() {
        let uut = TimerDescription::parse("30 11 * * *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::hours(23)).to_local(), result);
    }

    #[test]
    fn advance_by_one_day() {
        let uut = TimerDescription::parse("30 12 * * *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(1)).to_local(), result);
    }

    #[test]
    fn advance_by_two_days() {
        let uut = TimerDescription::parse("30 12 17 * *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(2)).to_local(), result);
    }

    #[test]
    fn advance_wrap_around_days() {
        let uut = TimerDescription::parse("30 12 14 * *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(29)).to_local(), result);
    }

    #[test]
    fn advance_by_one_month() {
        let uut = TimerDescription::parse("30 12 15 * *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(30)).to_local(), result);
    }

    #[test]
    fn advance_by_two_months() {
        let uut = TimerDescription::parse("30 12 15 8 *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(30 + 31)).to_local(), result);
    }

    #[test]
    fn advance_wrap_around_months() {
        let uut = TimerDescription::parse("30 12 15 6 *");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(365)).to_local(), result);
    }

    #[test]
    fn advance_by_one_weekday() {
        let uut = TimerDescription::parse("30 12 * * 2");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(1)).to_local(), result);
    }

    #[test]
    fn advance_by_two_weekdays() {
        let uut = TimerDescription::parse("30 12 * * 3");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(2)).to_local(), result);
    }

    #[test]
    fn advance_wrap_around_weekdays() {
        let uut = TimerDescription::parse("30 12 * * 0");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(6)).to_local(), result);
    }

    #[test]
    fn advance_with_weekday_taking_precedence() {
        let uut = TimerDescription::parse("30 12 17 6 2");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(1)).to_local(), result);
    }

    #[test]
    fn advance_with_date_taking_precedence() {
        let uut = TimerDescription::parse("30 12 16 6 3");

        let result = uut.unwrap().get_next_execution(mock_time());

        assert_eq!((mock_time() + Duration::days(1)).to_local(), result);
    }

    // Return 1970-06-15T12:30:00 CET Monday
    fn mock_time() -> Tm {
        time::at(Timespec::new(14297400, 0))
    }

}
