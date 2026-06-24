pub fn timestamp() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

pub fn to_int(value: &str, default: i32) -> i32 {
    value.trim().parse().unwrap_or(default)
}
