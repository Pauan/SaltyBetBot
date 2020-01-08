#[macro_export]
macro_rules! log {
    ($($args:tt)*) => {
        $crate::console_log(std::format!("{} [{}:{}] {}", $crate::current_date_pretty(), std::file!(), std::line!(), std::format!($($args)*)));
    };
}


#[macro_export]
macro_rules! time {
    ($name:expr, $value:expr) => {{
        let old = $crate::performance_now();
        let value = $value;
        let new = $crate::performance_now();
        $crate::log!("{} took {}ms", $name, new - old);
        value
    }}
}


#[macro_export]
macro_rules! server_log {
    ($($args:tt)*) => {
        $crate::api::server_log(std::format!("{} [{}:{}] {}", $crate::current_date_pretty(), std::file!(), std::line!(), std::format!($($args)*)))
    }
}
