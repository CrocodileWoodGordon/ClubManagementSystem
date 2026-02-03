use chrono::{DateTime, Local, NaiveDate, Utc};

pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

pub fn today_local() -> NaiveDate {
    Local::now().date_naive()
}
