use std::sync::atomic::{AtomicBool, Ordering};

static LOGGING_ENABLED: AtomicBool = AtomicBool::new(true);

/// ログの有効・無効を切り替えます。
pub fn set_enabled(enabled: bool) {
    LOGGING_ENABLED.store(enabled, Ordering::Relaxed);
}

/// ログが有効かどうかを返します。
pub fn is_enabled() -> bool {
    LOGGING_ENABLED.load(Ordering::Relaxed)
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        if $crate::logger::is_enabled() {
            println!($($arg)*);
        }
    };
}
