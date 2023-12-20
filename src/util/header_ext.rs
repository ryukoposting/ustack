use std::str::Split;

use chrono::{FixedOffset, DateTime, TimeZone};
use hyper::{HeaderMap, header::{HeaderValue, IF_MODIFIED_SINCE, CACHE_CONTROL}};

pub trait HeaderExt {
    fn if_modified_since(&self) -> Option<IfModifiedSince>;
    fn cache_control<'a>(&'a self) -> Option<CacheControl<'a>>;
    fn accepted_manipulations<'a>(&'a self) -> Option<AcceptedManipulations<'a>>;

    fn is_cache_valid<TZ>(&self, current: &DateTime<TZ>) -> bool
    where
        TZ: TimeZone
    {
        let no_cache = self.cache_control()
            .map_or(false, |cc| cc.is_no_cache());
        let cache_valid = self.if_modified_since()
            .map_or(false, |ifs| ifs.is_up_to_date(current));

        cache_valid && !no_cache
    }
}

pub struct IfModifiedSince(DateTime<FixedOffset>);
impl IfModifiedSince {
    fn is_up_to_date<TZ>(&self, current: &DateTime<TZ>) -> bool
    where
        TZ: TimeZone
    {
        current <= &(self.0 + chrono::Duration::seconds(1))
    }

    pub fn as_datetime(&self) -> &DateTime<FixedOffset> {
        &self.0
    }
}


pub struct CacheControl<'a>(Split<'a, [char; 19]>);
impl<'a> CacheControl<'a> {
    pub fn is_no_cache(&self) -> bool {
        self.0.clone().any(|token| token == "no-cache")
    }
}

pub struct AcceptedManipulations<'a>(Split<'a, [char; 19]>);
impl<'a> AcceptedManipulations<'a> {
    pub fn includes_feed(&self) -> bool {
        self.0.clone().any(|token| token == "feed")
    }
}


impl HeaderExt for HeaderMap<HeaderValue> {
    fn if_modified_since(&self) -> Option<IfModifiedSince> {
        let value = self.get(IF_MODIFIED_SINCE)?;
        let text = value.to_str().ok()?;
        let date = DateTime::parse_from_rfc2822(text).ok()?;
        Some(IfModifiedSince(date))
    }

    fn cache_control<'a>(&'a self) -> Option<CacheControl<'a>> {
        let value = self.get(CACHE_CONTROL)?;
        let text = value.to_str().ok()?;
        let spl = text.split(SEPARATORS);
        Some(CacheControl(spl))
    }

    fn accepted_manipulations<'a>(&'a self) -> Option<AcceptedManipulations<'a>> {
        let value = self.get("A-IM")?;
        let text = value.to_str().ok()?;
        let spl = text.split(SEPARATORS);
        Some(AcceptedManipulations(spl))
    }
}

// Header value 'separators' according to RFC 2616
const SEPARATORS: [char; 19] = [
    '(', ')', '<', '>', '@', ',', ';', ':', '\\', '"',
    '/', '[', ']', '?', '=', '{', '}', ' ', '\t'
];

// Header value 'control characters' according to RFC 2616
fn is_ctl(c: char) -> bool {
    c.is_ascii() && (c as u8 > 31) && (c as u8 != 127)
}
