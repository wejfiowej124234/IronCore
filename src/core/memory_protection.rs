//! Provides functions to lock and unlock memory, preventing it from being swapped to disk.
//! This is a security measure to protect sensitive data like private keys.
//!
//! It uses `libc::mlock` on Unix-like systems and `VirtualLock` on Windows.
//! A no-op fallback is provided for other platforms to ensure cross-platform compatibility.

// Conditional imports for platform-specific functionality under the "memlock" feature.
#[cfg(all(unix, feature = "memlock"))]
use libc;
#[cfg(all(windows, feature = "memlock"))]
use winapi::shared::minwindef::LPVOID;
#[cfg(all(windows, feature = "memlock"))]
use winapi::um::memoryapi::{VirtualLock, VirtualUnlock};

/// Locks a region of memory on Unix to prevent it from being swapped to disk.
///
/// On non-Unix platforms, this is a no-op and always returns `Ok(())`.
#[cfg(all(unix, feature = "memlock"))]
pub fn lock_memory(ptr: *const u8, len: usize) -> Result<(), std::io::Error> {
    // safety: The caller must ensure that `ptr` and `len` define a valid memory region.
    // Single unsafe expression for the actual FFI call
    let res = unsafe { libc::mlock(ptr as *const std::ffi::c_void, len as libc::size_t) };
    if res != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Unlocks a region of memory on Unix, allowing it to be swapped to disk again.
///
/// On non-Unix platforms, this is a no-op and always returns `Ok(())`.
#[cfg(all(unix, feature = "memlock"))]
pub fn unlock_memory(ptr: *const u8, len: usize) -> Result<(), std::io::Error> {
    let res = unsafe { libc::munlock(ptr as *const std::ffi::c_void, len as libc::size_t) };
    if res != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
/// Locks a region of memory on Windows to prevent it from being swapped to disk.
#[cfg(all(windows, feature = "memlock"))]
pub fn lock_memory(ptr: *const u8, len: usize) -> Result<(), std::io::Error> {
    // Single unsafe expression for the actual FFI call
    let res = unsafe { VirtualLock(ptr as LPVOID, len) };
    if res == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Unlocks a region of memory on Windows, allowing it to be swapped to disk again.
#[cfg(all(windows, feature = "memlock"))]
pub fn unlock_memory(ptr: *const u8, len: usize) -> Result<(), std::io::Error> {
    let res = unsafe { VirtualUnlock(ptr as LPVOID, len) };
    if res == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

// Provide no-op implementations for platforms where memlock is not supported or not enabled.
#[cfg(not(feature = "memlock"))]
pub fn lock_memory(_ptr: *const u8, _len: usize) -> Result<(), std::io::Error> {
    // When the "memlock" feature is not enabled, this is a no-op.
    Ok(())
}

#[cfg(not(feature = "memlock"))]
pub fn unlock_memory(_ptr: *const u8, _len: usize) -> Result<(), std::io::Error> {
    // When the "memlock" feature is not enabled, this is a no-op.
    Ok(())
}
