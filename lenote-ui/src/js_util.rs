use js_sys::{Date, Number};

pub fn get_js_date_string(timestamp: i64) -> String {
    return String::from(Date::new(&Number::from(timestamp as f64 * 1000.0)).to_string());
}

pub fn now() -> i64 {
    (Date::new_0().get_time() / 1000.0) as i64
}
