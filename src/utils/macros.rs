// Outputs to stderr only when debug
#[macro_export]
macro_rules! deprintln {
    ($($rest:tt)*) => {
        #[cfg(debug_assertions)]
        std::eprintln!($($rest)*)
    }
}
