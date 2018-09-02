#[macro_export]
macro_rules! log {
    ($($args:tt)*) => {
        js! { @(no_return)
            console.log(@{format!($($args)*)});
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
