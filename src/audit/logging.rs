use tracing::{error, info};

pub fn init_logger() {
    let _ = ();
}

pub fn log_operation(op: &str, user_id: &str, success: bool) {
    if success {
        info!(operation = %op, user_id = %user_id, "operation ok");
    } else {
        error!(operation = %op, user_id = %user_id, "operation failed");
    }
}
