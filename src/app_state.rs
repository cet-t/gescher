use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static IS_ACTIVE: AtomicBool = AtomicBool::new(true);
static IGNORE_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn is_active() -> bool {
    IS_ACTIVE.load(Ordering::Relaxed)
}

pub fn set_active(active: bool) {
    IS_ACTIVE.store(active, Ordering::Relaxed);
}

pub fn should_ignore() -> bool {
    let count = IGNORE_COUNT.load(Ordering::Relaxed);
    if count > 0 {
        IGNORE_COUNT.store(count - 1, Ordering::Relaxed);
        true
    } else {
        false
    }
}

pub fn add_ignore_count(count: usize) {
    IGNORE_COUNT.fetch_add(count, Ordering::Relaxed);
}

pub fn get_active_window_process_path() -> Option<String> {
    match active_win_pos_rs::get_active_window() {
        Ok(window) => Some(window.process_path.to_string_lossy().to_string()),
        Err(_) => None,
    }
}
