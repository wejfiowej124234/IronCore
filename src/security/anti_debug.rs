// ...existing code...
//! Anti-debugging functionality
//!
//! This module provides tools to detect if the application is being run under a debugger,
//! which can be used as part of security measures against reverse engineering attempts.

use tracing::warn;

/// Checks if the current process is being run under a debugger
///
/// # Returns
/// `true` if a debugger is detected, `false` otherwise
///
/// # Platform Support
/// - Windows: Uses IsDebuggerPresent API
/// - Linux: Checks TracerPid in /proc/self/status
/// - macOS: Uses ptrace with PT_DENY_ATTACH
/// - Other platforms: Returns false (not implemented)
pub fn is_debugger_present() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Diagnostics::Debug::IsDebuggerPresent;

        // The `as_bool()` method on `BOOL` is a safe conversion.
        let result = unsafe { IsDebuggerPresent().as_bool() };
        if result {
            warn!("Debugger detected on Windows platform");
        }
        result
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        // Check for TracerPid in /proc/self/status
        // If the file can't be opened or read, we can't detect a debugger, so we default to false.
        if let Ok(file) = File::open("/proc/self/status") {
            let reader = BufReader::new(file);
            // Use map_while(Result::ok) so we stop if an Err is produced instead of looping forever.
            for line in reader.lines().map_while(Result::ok) {
                if line.starts_with("TracerPid:") {
                    if let Some(pid_str) = line.split_whitespace().nth(1) {
                        if pid_str != "0" {
                            warn!("Debugger detected on Linux platform (TracerPid: {})", pid_str);
                            return true;
                        }
                    }
                    // We found the line, no need to continue.
                    return false;
                }
            }
        }
        // Default to false if we can't determine.
        false
    }

    #[cfg(target_os = "macos")]
    {
        use std::ptr;

        // On macOS, use ptrace to detect debuggers
        #[allow(non_camel_case_types)]
        type pid_t = i32;

        const PT_DENY_ATTACH: i32 = 31;

        extern "C" {
            fn ptrace(request: i32, pid: pid_t, addr: *mut std::ffi::c_void, data: i32) -> i32;
        }

        // Try to prevent a debugger from attaching.
        // If this fails (returns -1), it might indicate a debugger is already present.
        let result = unsafe { ptrace(PT_DENY_ATTACH, 0, ptr::null_mut(), 0) != 0 };

        if result {
            warn!("Debugger detected on macOS platform");
        }

        result
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        // Other platforms not supported yet
        warn!("Debugger detection is not supported on this platform.");
        false
    }
}
// ...existing code...
