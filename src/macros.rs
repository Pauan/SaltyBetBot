#[macro_export]
macro_rules! log {
    ($($args:tt)*) => {
        js! { @(no_return)
            console.log(@{format!("{} [{}:{}] {}", $crate::current_date_pretty(), file!(), line!(), format!($($args)*))});
        }
    };
}


#[macro_export]
macro_rules! time {
    ($name:expr, $value:expr) => {{
        let old = $crate::performance_now();
        let value = $value;
        let new = $crate::performance_now();
        log!("{} took {}ms", $name, new - old);
        value
    }}
}


#[macro_export]
macro_rules! server_log {
    ($($args:tt)*) => {
        $crate::server_log(format!("{} [{}:{}] {}", $crate::current_date_pretty(), file!(), line!(), format!($($args)*)))
    }
}
