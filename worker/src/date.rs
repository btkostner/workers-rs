use std::fmt::Display;

use js_sys::Date as JsDate;
use wasm_bindgen::JsValue;

/// The equivalent to a JavaScript `Date` Object.
/// ```no_run
/// # use worker::{Date, DateInit};
/// let now = Date::now();
/// let millis = now.as_millis();
/// // or use a specific point in time:
/// let t1: Date = DateInit::Millis(1630611511000).into();
/// let t2: Date = DateInit::String("Thu, 02 Sep 2021 19:38:31 GMT".to_string()).into();
/// ```
#[derive(Debug, Clone, Eq)]
pub struct Date {
    js_date: JsDate,
}

impl PartialEq for Date {
    fn eq(&self, other: &Self) -> bool {
        self.js_date.as_f64() == other.js_date.as_f64()
    }
}

/// Initialize a `Date` by constructing this enum.
/// ```no_run
/// # use worker::{Date, DateInit};
/// let t1: Date = DateInit::Millis(1630611511000).into();
/// let t2: Date = DateInit::String("Thu, 02 Sep 2021 19:38:31 GMT".to_string()).into();
/// ```
#[derive(Debug)]
pub enum DateInit {
    Millis(u64),
    String(String),
}

impl From<DateInit> for Date {
    fn from(init: DateInit) -> Self {
        Date::new(init)
    }
}

impl Date {
    /// Create a new Date, which requires being initialized from a known DateInit value.
    pub fn new(init: DateInit) -> Self {
        let val = match init {
            DateInit::Millis(n) => JsValue::from_f64(n as f64),
            DateInit::String(s) => JsValue::from_str(&s),
        };

        Self {
            js_date: JsDate::new(&val),
        }
    }

    /// Get the current time, represented by a Date.
    pub fn now() -> Self {
        Self {
            js_date: JsDate::new_0(),
        }
    }

    /// Convert a Date into its number of milliseconds since the Unix epoch.
    pub fn as_millis(&self) -> u64 {
        self.js_date.get_time() as u64
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.js_date.to_string())
    }
}

// Jiff truncates negative fractional milliseconds towards zero; JavaScript
// dates and alarms need the containing millisecond, including before the epoch.
pub(crate) fn timestamp_millis(timestamp: jiff::Timestamp) -> i64 {
    timestamp.as_nanosecond().div_euclid(1_000_000) as i64
}

impl From<jiff::Timestamp> for Date {
    fn from(timestamp: jiff::Timestamp) -> Self {
        Self {
            js_date: JsDate::new(&JsValue::from_f64(timestamp_millis(timestamp) as f64)),
        }
    }
}

impl From<jiff::ZonedDateTime> for Date {
    fn from(datetime: jiff::ZonedDateTime) -> Self {
        datetime.timestamp().into()
    }
}

/// Interpret a civil date as midnight UTC, failing outside Jiff's timestamp range.
impl TryFrom<jiff::civil::Date> for Date {
    type Error = crate::Error;

    fn try_from(date: jiff::civil::Date) -> crate::Result<Self> {
        Ok(jiff::tz::Offset::UTC.to_timestamp(date.at(0, 0, 0, 0))?.into())
    }
}

/// Invalid JavaScript dates and dates outside Jiff's timestamp range return an error.
impl TryFrom<&Date> for jiff::Timestamp {
    type Error = crate::Error;

    fn try_from(date: &Date) -> crate::Result<Self> {
        let millis = date.js_date.get_time();
        if !millis.is_finite() {
            return Err(crate::Error::RustError("Invalid date".into()));
        }
        Ok(Self::from_millisecond(millis as i64)?)
    }
}

impl TryFrom<Date> for jiff::Timestamp {
    type Error = crate::Error;

    fn try_from(date: Date) -> crate::Result<Self> {
        Self::try_from(&date)
    }
}

impl From<Date> for JsDate {
    fn from(val: Date) -> Self {
        val.js_date
    }
}

#[cfg(test)]
mod tests {
    use super::{timestamp_millis, Date, DateInit, JsDate};
    use jiff::Timestamp;
    use wasm_bindgen::JsValue;

    #[test]
    fn millisecond_conversion_floors_before_epoch() {
        let timestamp = Timestamp::new(-1, 999_500_000).unwrap();
        assert_eq!(timestamp_millis(timestamp), -1);

        let timestamp = Timestamp::new(0, 999_500_000).unwrap();
        assert_eq!(timestamp_millis(timestamp), 999);
    }

    #[test]
    fn millisecond_conversion_preserves_exact_negative_milliseconds() {
        let timestamp = Timestamp::from_millisecond(-1).unwrap();
        assert_eq!(timestamp_millis(timestamp), -1);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn invalid_and_out_of_range_js_dates_are_rejected() {
        let invalid = Date::from(JsDate::new(&JsValue::from_f64(f64::NAN)));
        assert!(Timestamp::try_from(invalid).is_err());

        let out_of_range = Date::new(DateInit::Millis(8_640_000_000_000_000));
        assert!(Timestamp::try_from(out_of_range).is_err());
    }
}


impl From<JsDate> for Date {
    fn from(js_date: JsDate) -> Self {
        Self { js_date }
    }
}
