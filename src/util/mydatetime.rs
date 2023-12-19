use chrono::{DateTime, FixedOffset, Local, format::{DelayedFormat, StrftimeItems}};
use std::{
    fmt::{Debug, Display},
    ops::Deref, time::SystemTime,
};

#[derive(Debug, PartialEq, Clone)]
pub struct MyDateTime(DateTime<FixedOffset>);

const DISPLAY_FORMAT: &str = "%e %b %Y %H:%M:%S %z";
const NO_SECONDS_24_FORMAT: &str = "%e %b %Y %H:%M %z";

const ALLOWED_FORMATS: [&'static str; 8] = [
    "%e %b %Y %I:%M:%S %p %z",
    DISPLAY_FORMAT,
    "%e %B %Y %I:%M:%S %p %z",
    "%e %B %Y %H:%M:%S %z",
    "%e %b %Y %I:%M %p %z",
    NO_SECONDS_24_FORMAT,
    "%e %B %Y %I:%M %p %z",
    "%e %B %Y %H:%M %z",
];

impl MyDateTime {
    pub fn now() -> Self {
        Local::now().into()
    }

    pub fn to_string_no_seconds(&self) -> DelayedFormat<StrftimeItems> {
        self.0.format(NO_SECONDS_24_FORMAT)
    }

    pub fn to_string_rss(&self) -> String {
        self.0.to_rfc2822()
    }

    pub fn system_time(&self) -> SystemTime {
        SystemTime::from(self.0)
    }
}

impl Deref for MyDateTime {
    type Target = DateTime<FixedOffset>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialOrd for MyDateTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<'de> serde::Deserialize<'de> for MyDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let s = String::deserialize(deserializer)?;

        for format in ALLOWED_FORMATS.iter() {
            let parsed = DateTime::<FixedOffset>::parse_from_str(&s, format);
            if let Ok(parsed) = parsed {
                return Ok(MyDateTime(parsed.into()));
            }
        }

        Err(D::Error::custom(format!("Invalid datetime format")))
    }
}

impl Display for MyDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted = self.0.format(DISPLAY_FORMAT);
        std::fmt::Display::fmt(&formatted, f)
    }
}

impl From<DateTime<Local>> for MyDateTime {
    fn from(value: DateTime<Local>) -> Self {
        Self(value.fixed_offset())
    }
}

impl From<SystemTime> for MyDateTime {
    fn from(value: SystemTime) -> Self {
        Self::from(DateTime::<Local>::from(value))
    }
}

#[cfg(test)]
mod test {
    use super::ALLOWED_FORMATS;
    use chrono::{DateTime, FixedOffset};

    #[test]
    fn parse_format_1() {
        let input = "28 Aug 2023 06:00:00 PM +0500";

        let parsed = DateTime::<FixedOffset>::parse_from_str(input, ALLOWED_FORMATS[0]);
        if let Err(err) = &parsed {
            eprintln!("{}", err);
        }

        parsed.expect("parse_from_str");
    }

    #[test]
    fn parse_format_2() {
        let input = "28 Aug 2023 18:00:00 +0500";

        let parsed = DateTime::<FixedOffset>::parse_from_str(input, ALLOWED_FORMATS[1]);
        if let Err(err) = &parsed {
            eprintln!("{}", err);
        }

        parsed.expect("parse_from_str");
    }

    #[test]
    fn parse_format_3() {
        let input = "28 August 2023 06:00:00 PM +0500";

        let parsed = DateTime::<FixedOffset>::parse_from_str(input, ALLOWED_FORMATS[2]);
        if let Err(err) = &parsed {
            eprintln!("{}", err);
        }

        parsed.expect("parse_from_str");
    }

    #[test]
    fn parse_format_4() {
        let input = "28 August 2023 18:00:00 +0500";

        let parsed = DateTime::<FixedOffset>::parse_from_str(input, ALLOWED_FORMATS[3]);
        if let Err(err) = &parsed {
            eprintln!("{}", err);
        }

        parsed.expect("parse_from_str");
    }

    #[test]
    fn parse_format_5() {
        let input = "28 Aug 2023 06:00 PM +0500";

        let parsed = DateTime::<FixedOffset>::parse_from_str(input, ALLOWED_FORMATS[4]);
        if let Err(err) = &parsed {
            eprintln!("{}", err);
        }

        parsed.expect("parse_from_str");
    }

    #[test]
    fn parse_format_6() {
        let input = "28 Aug 2023 18:00 +0500";

        let parsed = DateTime::<FixedOffset>::parse_from_str(input, ALLOWED_FORMATS[5]);
        if let Err(err) = &parsed {
            eprintln!("{}", err);
        }

        parsed.expect("parse_from_str");
    }

    #[test]
    fn parse_format_7() {
        let input = "28 August 2023 06:00 PM +0500";

        let parsed = DateTime::<FixedOffset>::parse_from_str(input, ALLOWED_FORMATS[6]);
        if let Err(err) = &parsed {
            eprintln!("{}", err);
        }

        parsed.expect("parse_from_str");
    }

    #[test]
    fn parse_format_8() {
        let input = "28 August 2023 18:00 +0500";

        let parsed = DateTime::<FixedOffset>::parse_from_str(input, ALLOWED_FORMATS[7]);
        if let Err(err) = &parsed {
            eprintln!("{}", err);
        }

        parsed.expect("parse_from_str");
    }
}
