use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike, Weekday};
use serde::{Deserialize, Deserializer};
use std::fmt::Formatter;
use std::str::FromStr;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum SaveInterval {
    /// every **:\[012345]\[05]:00 UTC
    Every5Minute,
    /// every **:\[012345]0:00 UTC
    Every10Minute,
    /// every **:(00|15|30|45):00 UTC
    Every15Minute,
    /// every **:\[024]0:00 UTC
    Every20Minute,
    /// every **:\[03]0:00 UTC
    /// alias: half-hourly
    Every30Minute,
    /// every **:00:00 UTC
    Every1Hour,
    /// every \[0-2]\[02468]:00:00 UTC
    Every2Hour,
    /// every (00|04|08|12|16|20):00:00 UTC
    Every4Hour,
    /// every (00|06|12|18):00:00 UTC
    Every6Hour,
    /// every (00|08|16):00:00 UTC
    Every8Hour,
    /// every (00|12):00:00 UTC
    /// alias: half-daily
    Every12Hour,
    /// every 00:00:00 UTC
    // alias: 24 hour
    Every1Day,
    /// every Monday 00:00:00 UTC
    Every1Week,
    /// every 1st 00:00:00 UTC
    Every1Month,
    /// every (Jan|Mar|May|Jul|Sep|Nov) 1st 00:00:00 UTC
    Every2Month,
    /// every (Jan|Apr|Jul|Oct) 1st 00:00:00 UTC
    Every3Month,
    /// every (Jan|May|Nov) 1st 00:00:00 UTC
    Every4Month,
    /// every (Jan|Jun) 1st 00:00:00 UTC
    /// alias: half-year
    Every6Month,
    /// every Jan 1st 00:00:00 UTC
    Every1Year,
}

impl<'de> Deserialize<'de> for SaveInterval {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VisitorImpl;

        impl<'de> serde::de::Visitor<'de> for VisitorImpl {
            type Value = SaveInterval;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                write!(formatter, "expecting interval specifier")
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                SaveInterval::from_str(v).map_err(|e| E::custom(e))
            }
        }

        deserializer.deserialize_str(VisitorImpl)
    }
}

impl std::str::FromStr for SaveInterval {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Parser {
            src: s.as_bytes(),
            index: 0,
        }
        .parse()
    }
}

impl std::fmt::Display for SaveInterval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Every1Year => write!(f, "every 1 year"),
            Self::Every6Month => write!(f, "every 6 month"),
            Self::Every4Month => write!(f, "every 4 month"),
            Self::Every3Month => write!(f, "every 3 month"),
            Self::Every2Month => write!(f, "every 2 month"),
            Self::Every1Month => write!(f, "every 1 month"),
            Self::Every1Week => write!(f, "every 1 week"),
            Self::Every1Day => write!(f, "every 1 day"),
            Self::Every12Hour => write!(f, "every 12 hour"),
            Self::Every8Hour => write!(f, "every 8 hour"),
            Self::Every6Hour => write!(f, "every 6 hour"),
            Self::Every4Hour => write!(f, "every 4 hour"),
            Self::Every2Hour => write!(f, "every 2 hour"),
            Self::Every1Hour => write!(f, "every 1 hour"),
            Self::Every30Minute => write!(f, "every 30 minute"),
            Self::Every20Minute => write!(f, "every 20 minute"),
            Self::Every15Minute => write!(f, "every 15 minute"),
            Self::Every10Minute => write!(f, "every 10 minute"),
            Self::Every5Minute => write!(f, "every 5 minute"),
        }
    }
}

impl SaveInterval {
    pub(crate) fn is_passed(self, since: &NaiveDateTime, until: &NaiveDateTime) -> bool {
        debug_assert!(since < until);

        //since.time().num_seconds_from_midnight() / 300

        macro_rules! compare {
            ($method: ident / $per: expr) => {
                since.$method() / $per != until.$method() / $per
            };
        }
        macro_rules! compare_date {
            ($per_sec: expr) => {
                compare!(num_seconds_from_midnight / $per_sec)
            };
        }

        match self {
            SaveInterval::Every5Minute => compare_date!(60 * 5),
            SaveInterval::Every10Minute => compare_date!(60 * 10),
            SaveInterval::Every15Minute => compare_date!(60 * 15),
            SaveInterval::Every20Minute => compare_date!(60 * 20),
            SaveInterval::Every30Minute => compare_date!(60 * 30),
            SaveInterval::Every1Hour => compare_date!(60 * 60 * 1),
            SaveInterval::Every2Hour => compare_date!(60 * 60 * 2),
            SaveInterval::Every4Hour => compare_date!(60 * 60 * 4),
            SaveInterval::Every6Hour => compare_date!(60 * 60 * 6),
            SaveInterval::Every8Hour => compare_date!(60 * 60 * 8),
            SaveInterval::Every12Hour => compare_date!(60 * 60 * 12),
            SaveInterval::Every1Day => since.date() != until.date(),
            SaveInterval::Every1Week => since.iso_week() != until.iso_week(),
            SaveInterval::Every1Month => compare!(month0 / 1),
            SaveInterval::Every2Month => compare!(month0 / 2),
            SaveInterval::Every3Month => compare!(month0 / 3),
            SaveInterval::Every4Month => compare!(month0 / 4),
            SaveInterval::Every6Month => compare!(month0 / 6),
            SaveInterval::Every1Year => compare!(year / 1),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn get_last_date_until(self, time: &NaiveDateTime) -> NaiveDateTime {
        //noinspection SpellCheckingInspection
        /// returns greatest multiple of m less than or equal to num
        /// for div opimization
        #[inline(always)]
        fn gmon<T>(num: T, m: T) -> T
        where
            T: std::ops::Rem<Output = T> + std::ops::Sub<Output = T> + Copy,
        {
            num - num % m
        }

        match self {
            SaveInterval::Every5Minute => {
                time.date().and_hms(time.hour(), gmon(time.minute(), 5), 0)
            }
            SaveInterval::Every10Minute => {
                time.date().and_hms(time.hour(), gmon(time.minute(), 10), 0)
            }
            SaveInterval::Every15Minute => {
                time.date().and_hms(time.hour(), gmon(time.minute(), 15), 0)
            }
            SaveInterval::Every20Minute => {
                time.date().and_hms(time.hour(), gmon(time.minute(), 20), 0)
            }
            SaveInterval::Every30Minute => {
                time.date().and_hms(time.hour(), gmon(time.minute(), 30), 0)
            }
            SaveInterval::Every1Hour => time.date().and_hms(gmon(time.hour(), 1), 0, 0),
            SaveInterval::Every2Hour => time.date().and_hms(gmon(time.hour(), 2), 0, 0),
            SaveInterval::Every4Hour => time.date().and_hms(gmon(time.hour(), 4), 0, 0),
            SaveInterval::Every6Hour => time.date().and_hms(gmon(time.hour(), 6), 0, 0),
            SaveInterval::Every8Hour => time.date().and_hms(gmon(time.hour(), 8), 0, 0),
            SaveInterval::Every12Hour => time.date().and_hms(gmon(time.hour(), 12), 0, 0),
            SaveInterval::Every1Day => time.date().and_hms(0, 0, 0),
            SaveInterval::Every1Week => {
                let week = time.iso_week();
                NaiveDate::from_isoywd(week.year(), week.week(), Weekday::Mon).and_hms(0, 0, 0)
            }
            SaveInterval::Every1Month => {
                NaiveDate::from_ymd(time.year(), time.month(), 1).and_hms(0, 0, 0)
            }
            SaveInterval::Every2Month => {
                NaiveDate::from_ymd(time.year(), gmon(time.month0(), 2) + 1, 1).and_hms(0, 0, 0)
            }
            SaveInterval::Every3Month => {
                NaiveDate::from_ymd(time.year(), gmon(time.month0(), 3) + 1, 1).and_hms(0, 0, 0)
            }
            SaveInterval::Every4Month => {
                NaiveDate::from_ymd(time.year(), gmon(time.month0(), 4) + 1, 1).and_hms(0, 0, 0)
            }
            SaveInterval::Every6Month => {
                NaiveDate::from_ymd(time.year(), gmon(time.month0(), 6) + 1, 1).and_hms(0, 0, 0)
            }
            SaveInterval::Every1Year => NaiveDate::from_ymd(time.year(), 1, 1).and_hms(0, 0, 0),
        }
    }
}

#[cfg(test)]
mod get_last_date_until_test {
    use super::*;
    use SaveInterval::*;

    #[test]
    fn get_last_date_until() {
        let date = NaiveDate::from_ymd(2022, 1, 2);
        let date_time = date.and_hms(3, 28, 30);

        assert_eq!(
            Every1Year.get_last_date_until(&date_time),
            NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
        );
        assert_eq!(
            Every6Month.get_last_date_until(&date_time),
            NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
        );
        assert_eq!(
            Every1Month.get_last_date_until(&date_time),
            NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
        );
        assert_eq!(
            Every1Week.get_last_date_until(&date_time),
            NaiveDate::from_ymd(2021, 12, 27).and_hms(0, 0, 0)
        );

        assert_eq!(
            Every1Day.get_last_date_until(&date_time),
            date.and_hms(0, 0, 0)
        );
        assert_eq!(
            Every12Hour.get_last_date_until(&date_time),
            date.and_hms(0, 0, 0)
        );
        assert_eq!(
            Every8Hour.get_last_date_until(&date_time),
            date.and_hms(0, 0, 0)
        );
        assert_eq!(
            Every6Hour.get_last_date_until(&date_time),
            date.and_hms(0, 0, 0)
        );
        assert_eq!(
            Every4Hour.get_last_date_until(&date_time),
            date.and_hms(0, 0, 0)
        );
        assert_eq!(
            Every2Hour.get_last_date_until(&date_time),
            date.and_hms(2, 0, 0)
        );
        assert_eq!(
            Every1Hour.get_last_date_until(&date_time),
            date.and_hms(3, 0, 0)
        );
        assert_eq!(
            Every30Minute.get_last_date_until(&date_time),
            date.and_hms(3, 0, 0)
        );
        assert_eq!(
            Every20Minute.get_last_date_until(&date_time),
            date.and_hms(3, 20, 0)
        );
        assert_eq!(
            Every15Minute.get_last_date_until(&date_time),
            date.and_hms(3, 15, 0)
        );
        assert_eq!(
            Every10Minute.get_last_date_until(&date_time),
            date.and_hms(3, 20, 0)
        );
        assert_eq!(
            Every5Minute.get_last_date_until(&date_time),
            date.and_hms(3, 25, 0)
        );
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    InvalidCharacter(usize),
    UnexpectedToken(String),
    Unsupported(String),
    NumberOverflow,
    Empty,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidCharacter(offset) => write!(f, "invalid character at {}", offset),
            Error::UnexpectedToken(token) if token.is_empty() => write!(
                f,
                "expected unit token. year, month, week, day, hour, and minute are allowed"
            ),
            Error::UnexpectedToken(token) => write!(f, "unknown token {:?}", token),
            Error::Unsupported(token) => write!(f, "unsupported interval: {:?}", token),
            Error::NumberOverflow => write!(f, "number is too large"),
            Error::Empty => write!(f, "value was empty"),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Token {
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
    Every,
    Half,
    Number(u32),
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Minute => write!(f, "minute"),
            Token::Hour => write!(f, "hour"),
            Token::Day => write!(f, "day"),
            Token::Week => write!(f, "week"),
            Token::Month => write!(f, "month"),
            Token::Year => write!(f, "year"),
            Token::Every => write!(f, "every"),
            Token::Half => write!(f, "half"),
            Token::Number(n) => write!(f, "{}", n),
        }
    }
}

struct Parser<'a> {
    src: &'a [u8],
    index: usize,
}

impl<'a> Parser<'a> {
    fn parse_keyword(&mut self) -> Result<Token, Error> {
        let begin = self.index;
        while matches!(self.src.get(self.index), Some(b'a'..=b'z' | b'A'..=b'Z')) {
            self.index += 1
        }
        match &self.src[begin..self.index] {
            b"minutely" | b"minutes" | b"minute" | b"min" | b"mins" | b"m" => Ok(Token::Minute),
            b"hourly" | b"hours" | b"hour" | b"hr" | b"hrs" | b"h" => Ok(Token::Hour),
            b"daily" | b"days" | b"day" | b"d" => Ok(Token::Day),
            b"weekly" | b"weeks" | b"week" | b"w" => Ok(Token::Week),
            b"monthly" | b"months" | b"month" | b"M" => Ok(Token::Month),
            b"yearly" | b"years" | b"year" | b"y" => Ok(Token::Year),

            b"half" => Ok(Token::Half),
            b"every" => Ok(Token::Every),

            e => Err(Error::UnexpectedToken(unsafe {
                String::from_utf8_unchecked(e.to_owned())
            })),
        }
    }

    fn parse_number(&mut self) -> Result<Token, Error> {
        let begin = self.index;
        while matches!(self.src.get(self.index), Some(b'0'..=b'9')) {
            self.index += 1
        }

        unsafe { std::str::from_utf8_unchecked(&self.src[begin..self.index]) }
            .parse::<u32>()
            .map(Token::Number)
            .map_err(|_| Error::NumberOverflow)
    }

    fn skip_ws(&mut self) -> bool {
        loop {
            match self
                .src
                .get(self.index)
                .map(|c| (*c as char).is_ascii_whitespace() || *c == b'-')
            {
                Some(true) => self.index += 1,
                None => break true,
                Some(false) => break false,
            }
        }
    }

    fn parse_token(&mut self) -> Result<Option<Token>, Error> {
        if self.skip_ws() {
            return Ok(None);
        }

        match self.src[self.index] {
            b'0'..=b'9' => self.parse_number().map(Some),
            b'a'..=b'z' | b'A'..=b'Z' => self.parse_keyword().map(Some),
            _ => Err(Error::InvalidCharacter(self.index)),
        }
    }

    fn parse(mut self) -> Result<SaveInterval, Error> {
        let mut t = self.parse_token()?.ok_or(Error::Empty)?;
        if t == Token::Every {
            t = self
                .parse_token()?
                .ok_or(Error::UnexpectedToken("every".to_owned()))?;
        }
        let interval = if t == Token::Half {
            t = self
                .parse_token()?
                .ok_or(Error::UnexpectedToken("half".to_owned()))?;
            match t {
                Token::Year => SaveInterval::Every6Month,
                Token::Day => SaveInterval::Every12Hour,
                Token::Hour => SaveInterval::Every30Minute,
                token => return Err(Error::Unsupported(format!("half {}", token))),
            }
        } else {
            let n = if let Token::Number(n) = t {
                t = self
                    .parse_token()?
                    .ok_or_else(|| Error::UnexpectedToken(n.to_string()))?;
                n
            } else {
                1
            };
            match (n, t) {
                (1, Token::Year) => SaveInterval::Every1Year,
                (6, Token::Month) => SaveInterval::Every6Month,
                (4, Token::Month) => SaveInterval::Every4Month,
                (3, Token::Month) => SaveInterval::Every3Month,
                (2, Token::Month) => SaveInterval::Every2Month,
                (1, Token::Month) => SaveInterval::Every1Month,
                (1, Token::Week) => SaveInterval::Every1Week,
                (1, Token::Day) => SaveInterval::Every1Day,
                (12, Token::Hour) => SaveInterval::Every12Hour,
                (8, Token::Hour) => SaveInterval::Every8Hour,
                (6, Token::Hour) => SaveInterval::Every6Hour,
                (4, Token::Hour) => SaveInterval::Every4Hour,
                (2, Token::Hour) => SaveInterval::Every2Hour,
                (1, Token::Hour) => SaveInterval::Every1Hour,
                (30, Token::Minute) => SaveInterval::Every30Minute,
                (20, Token::Minute) => SaveInterval::Every20Minute,
                (15, Token::Minute) => SaveInterval::Every15Minute,
                (10, Token::Minute) => SaveInterval::Every10Minute,
                (5, Token::Minute) => SaveInterval::Every5Minute,
                (_, Token::Every) => return Err(Error::UnexpectedToken("every".to_owned())),
                (_, Token::Half) => return Err(Error::UnexpectedToken("half".to_owned())),
                (_, Token::Number(_)) => return Err(Error::UnexpectedToken(String::new())),
                (n, token) => return Err(Error::Unsupported(format!("{} {}", n, token))),
            }
        };

        match self.parse_token()? {
            None => {}
            Some(t) => return Err(Error::UnexpectedToken(t.to_string())),
        }

        return Ok(interval);
    }
}

#[cfg(test)]
mod parse_test {
    use super::*;
    use SaveInterval::*;

    fn parse(str: &str) -> SaveInterval {
        str.parse().unwrap()
    }

    #[test]
    fn formal() {
        assert_eq!(parse("every 1 year"), Every1Year);
        assert_eq!(parse("every 6 month"), Every6Month);
        assert_eq!(parse("every 1 month"), Every1Month);
        assert_eq!(parse("every 1 week"), Every1Week);
        assert_eq!(parse("every 1 day"), Every1Day);
        assert_eq!(parse("every 12 hour"), Every12Hour);
        assert_eq!(parse("every 8 hour"), Every8Hour);
        assert_eq!(parse("every 6 hour"), Every6Hour);
        assert_eq!(parse("every 4 hour"), Every4Hour);
        assert_eq!(parse("every 2 hour"), Every2Hour);
        assert_eq!(parse("every 1 hour"), Every1Hour);
        assert_eq!(parse("every 30 minute"), Every30Minute);
        assert_eq!(parse("every 20 minute"), Every20Minute);
        assert_eq!(parse("every 15 minute"), Every15Minute);
        assert_eq!(parse("every 10 minute"), Every10Minute);
        assert_eq!(parse("every 5 minute"), Every5Minute);
    }

    #[test]
    fn no_one() {
        assert_eq!(parse("every year"), Every1Year);
        assert_eq!(parse("every month"), Every1Month);
        assert_eq!(parse("every week"), Every1Week);
        assert_eq!(parse("every day"), Every1Day);
        assert_eq!(parse("every hour"), Every1Hour);
    }

    #[test]
    fn with_minus() {
        assert_eq!(parse("every-1-year"), Every1Year);
        assert_eq!(parse("every-6-month"), Every6Month);
        assert_eq!(parse("every-1-month"), Every1Month);
        assert_eq!(parse("every-1-week"), Every1Week);
        assert_eq!(parse("every-1-day"), Every1Day);
        assert_eq!(parse("every-12-hour"), Every12Hour);
        assert_eq!(parse("every-8-hour"), Every8Hour);
        assert_eq!(parse("every-6-hour"), Every6Hour);
        assert_eq!(parse("every-4-hour"), Every4Hour);
        assert_eq!(parse("every-2-hour"), Every2Hour);
        assert_eq!(parse("every-1-hour"), Every1Hour);
        assert_eq!(parse("every-30-minute"), Every30Minute);
        assert_eq!(parse("every-20-minute"), Every20Minute);
        assert_eq!(parse("every-15-minute"), Every15Minute);
        assert_eq!(parse("every-10-minute"), Every10Minute);
        assert_eq!(parse("every-5-minute"), Every5Minute);

        assert_eq!(parse("every-year"), Every1Year);
        assert_eq!(parse("every-month"), Every1Month);
        assert_eq!(parse("every-week"), Every1Week);
        assert_eq!(parse("every-day"), Every1Day);
        assert_eq!(parse("every-hour"), Every1Hour);
    }

    #[test]
    fn no_space() {
        assert_eq!(parse("every1year"), Every1Year);
        assert_eq!(parse("every6month"), Every6Month);
        assert_eq!(parse("every1month"), Every1Month);
        assert_eq!(parse("every1week"), Every1Week);
        assert_eq!(parse("every1day"), Every1Day);
        assert_eq!(parse("every12hour"), Every12Hour);
        assert_eq!(parse("every8hour"), Every8Hour);
        assert_eq!(parse("every6hour"), Every6Hour);
        assert_eq!(parse("every4hour"), Every4Hour);
        assert_eq!(parse("every2hour"), Every2Hour);
        assert_eq!(parse("every1hour"), Every1Hour);
        assert_eq!(parse("every30minute"), Every30Minute);
        assert_eq!(parse("every20minute"), Every20Minute);
        assert_eq!(parse("every15minute"), Every15Minute);
        assert_eq!(parse("every10minute"), Every10Minute);
        assert_eq!(parse("every5minute"), Every5Minute);
    }

    #[test]
    fn no_every() {
        assert_eq!(parse("1 year"), Every1Year);
        assert_eq!(parse("6 month"), Every6Month);
        assert_eq!(parse("1 month"), Every1Month);
        assert_eq!(parse("1 week"), Every1Week);
        assert_eq!(parse("1 day"), Every1Day);
        assert_eq!(parse("12 hour"), Every12Hour);
        assert_eq!(parse("8 hour"), Every8Hour);
        assert_eq!(parse("6 hour"), Every6Hour);
        assert_eq!(parse("4 hour"), Every4Hour);
        assert_eq!(parse("2 hour"), Every2Hour);
        assert_eq!(parse("1 hour"), Every1Hour);
        assert_eq!(parse("30 minute"), Every30Minute);
        assert_eq!(parse("20 minute"), Every20Minute);
        assert_eq!(parse("15 minute"), Every15Minute);
        assert_eq!(parse("10 minute"), Every10Minute);
        assert_eq!(parse("5 minute"), Every5Minute);

        assert_eq!(parse("year"), Every1Year);
        assert_eq!(parse("month"), Every1Month);
        assert_eq!(parse("week"), Every1Week);
        assert_eq!(parse("day"), Every1Day);
        assert_eq!(parse("hour"), Every1Hour);
    }

    #[test]
    fn half() {
        assert_eq!(parse("half year"), Every6Month);
        assert_eq!(parse("half-year"), Every6Month);
        assert_eq!(parse("half day"), Every12Hour);
        assert_eq!(parse("half-day"), Every12Hour);
        assert_eq!(parse("half hour"), Every30Minute);
        assert_eq!(parse("half-hour"), Every30Minute);
    }

    #[test]
    fn test_ly() {
        assert_eq!(parse("1 yearly"), Every1Year);
        assert_eq!(parse("6 monthly"), Every6Month);
        assert_eq!(parse("1 monthly"), Every1Month);
        assert_eq!(parse("1 weekly"), Every1Week);
        assert_eq!(parse("1 daily"), Every1Day);
        assert_eq!(parse("12 hourly"), Every12Hour);
        assert_eq!(parse("8 hourly"), Every8Hour);
        assert_eq!(parse("6 hourly"), Every6Hour);
        assert_eq!(parse("4 hourly"), Every4Hour);
        assert_eq!(parse("2 hourly"), Every2Hour);
        assert_eq!(parse("1 hourly"), Every1Hour);
        assert_eq!(parse("30 minutely"), Every30Minute);
        assert_eq!(parse("20 minutely"), Every20Minute);
        assert_eq!(parse("15 minutely"), Every15Minute);
        assert_eq!(parse("10 minutely"), Every10Minute);
        assert_eq!(parse("5 minutely"), Every5Minute);

        assert_eq!(parse("yearly"), Every1Year);
        assert_eq!(parse("monthly"), Every1Month);
        assert_eq!(parse("weekly"), Every1Week);
        assert_eq!(parse("daily"), Every1Day);
        assert_eq!(parse("hourly"), Every1Hour);

        assert_eq!(parse("half yearly"), Every6Month);
        assert_eq!(parse("half-yearly"), Every6Month);
        assert_eq!(parse("half daily"), Every12Hour);
        assert_eq!(parse("half-daily"), Every12Hour);
        assert_eq!(parse("half hourly"), Every30Minute);
        assert_eq!(parse("half-hourly"), Every30Minute);
    }

    #[test]
    fn trim() {
        assert_eq!(parse("   every 1 year  "), Every1Year);
        assert_eq!(parse(" - every 1 year -"), Every1Year);
    }
}
