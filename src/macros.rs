#[macro_export]
macro_rules! log {
    ($($args:tt)*) => {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from(std::format!("{} [{}:{}] {}", $crate::current_date_pretty(), std::file!(), std::line!(), std::format!($($args)*))));
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
        $crate::server_log(std::format!("{} [{}:{}] {}", $crate::current_date_pretty(), std::file!(), std::line!(), std::format!($($args)*)))
    }
}
