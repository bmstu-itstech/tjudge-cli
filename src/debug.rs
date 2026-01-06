static VERBOSE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_verbose(enabled: bool) {
    VERBOSE.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(std::sync::atomic::Ordering::Relaxed)
}

#[macro_export]
macro_rules! vprintln {
    ($($arg:tt)*) => {
        if $crate::debug::is_verbose() {
            eprintln!($($arg)*);
        }
    };
}
