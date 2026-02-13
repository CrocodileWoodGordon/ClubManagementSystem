use chrono::{DateTime, Local, NaiveDate, Utc};

#[allow(dead_code)]
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

#[allow(dead_code)]
pub fn today_local() -> NaiveDate {
    Local::now().date_naive()
}
