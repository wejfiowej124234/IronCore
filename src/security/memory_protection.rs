// src/security/memory_protection.rs
//! 鍐呭瓨淇濇姢妯″潡
//! 鐢ㄤ簬瀹夊叏澶勭悊鏁忔劅鏁版嵁锛岄槻姝㈠唴瀹?
use crate::core::memory_protection::{lock_memory, unlock_memory};

use crate::core::errors::WalletError;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

/// 安全缓冲区：分配未初始化内存并在 Drop 时清零
pub struct SecureBuffer {
    ptr: *mut u8,
    len: usize,
    layout: Layout,
}

impl SecureBuffer {
    pub fn new(size: usize) -> Result<Self, WalletError> {
        if size == 0 {
            return Err(WalletError::InvalidInput("Buffer size cannot be zero".to_string()));
        }

        let layout = Layout::from_size_align(size, 8)
            .map_err(|_| WalletError::InvalidInput("Invalid buffer layout".to_string()))?;

        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err(WalletError::MemoryError("Failed to allocate secure memory".to_string()));
        }

        Ok(Self { ptr, len: size, layout })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), WalletError> {
        if data.len() > self.len {
            return Err(WalletError::InvalidInput("Data too large for buffer".to_string()));
        }
        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), self.ptr, data.len());
            // zero the rest (optional)
            if data.len() < self.len {
                ptr::write_bytes(self.ptr.add(data.len()), 0, self.len - data.len());
            }
        }
        Ok(())
    }

    pub fn read(&self, dest: &mut [u8]) -> Result<usize, WalletError> {
        let read_len = dest.len().min(self.len);
        unsafe {
            ptr::copy_nonoverlapping(self.ptr, dest.as_mut_ptr(), read_len);
        }
        Ok(read_len)
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    /// # Safety
    /// 调用者必须保证对返回的可变切片的使用不会违反所有权和别名规则。
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.ptr, self.len)
    }
}

impl Drop for SecureBuffer {
    fn drop(&mut self) {
        unsafe {
            clear_sensitive_data(self.ptr, self.len);
            dealloc(self.ptr, self.layout);
        }
    }
}

impl Clone for SecureBuffer {
    fn clone(&self) -> Self {
        if self.is_empty() {
            // ✅ 空buffer直接返回
            return Self::new(1).unwrap_or_else(|_| {
                panic!("Critical: Cannot allocate minimal buffer")
            });
        }
        let new_buf = Self::new(self.len).unwrap_or_else(|_| {
            panic!("Critical: Cannot clone SecureBuffer")
        });
        unsafe { ptr::copy_nonoverlapping(self.ptr, new_buf.ptr, self.len) };
        new_buf
    }
}

/// Clear敏感内存（尽量使用不可优化掉的写法）
///
/// # Safety
/// - `ptr` 必须指向可写的内存且长度至少为 `len`
pub unsafe fn clear_sensitive_data(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }

    // 首先用 volatile 写入 0，确保不会被优化掉
    for i in 0..len {
        std::ptr::write_volatile(ptr.add(i), 0u8);
    }

    // Memory fence to prevent reordering
    std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
}

/// 安全清零切片
pub fn clear_sensitive(buf: &mut [u8]) {
    unsafe {
        clear_sensitive_data(buf.as_mut_ptr(), buf.len());
    }
}

/// 安全字符串（基于 SecureBuffer）
pub struct SecureString {
    buffer: SecureBuffer,
}

impl SecureString {
    pub fn new(s: &str) -> Result<Self, WalletError> {
        if s.is_empty() {
            return Err(WalletError::InvalidInput("SecureString cannot be empty".to_string()));
        }
        let mut buffer = SecureBuffer::new(s.len())?;
        buffer.write(s.as_bytes())?;
        Ok(Self { buffer })
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.len() == 0
    }

    pub fn reveal(&self) -> Result<String, WalletError> {
        let mut data = vec![0u8; self.len()];
        self.buffer.read(&mut data)?;
        String::from_utf8(data)
            .map_err(|e| WalletError::InvalidInput(format!("Invalid UTF-8: {}", e)))
    }
}

impl Drop for SecureString {
    fn drop(&mut self) {
        // SecureBuffer 的 Drop 会清理底层内容
    }
}

/// 管理已锁定内存页面的简单分配器（示例实现）
pub struct SecureAllocator {
    locked_pages: Vec<(usize, usize)>, // (ptr, size)
}

impl SecureAllocator {
    pub fn new() -> Self {
        Self { locked_pages: Vec::new() }
    }

    pub fn alloc_locked(&mut self, size: usize) -> Result<SecureBuffer, WalletError> {
        let buffer = SecureBuffer::new(size)?;
        // call lock_memory (safe API) without unnecessary unsafe
        lock_memory(buffer.ptr, buffer.len())
            .map_err(|e| WalletError::MemoryError(e.to_string()))?;
        self.locked_pages.push((buffer.ptr as usize, buffer.len));
        Ok(buffer)
    }

    pub fn unlock_all(&mut self) -> Result<(), WalletError> {
        for (ptr, size) in &self.locked_pages {
            // call unlock_memory (safe API) without unnecessary unsafe
            unlock_memory(*ptr as *mut u8, *size)
                .map_err(|e| WalletError::MemoryError(e.to_string()))?;
        }
        self.locked_pages.clear();
        Ok(())
    }
}

impl Default for SecureAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SecureAllocator {
    fn drop(&mut self) {
        let _ = self.unlock_all();
    }
}

/// 短期敏感数据包装：Drop 时执行传入的清理函数
pub struct TempSensitive<T, F>
where
    F: FnMut(&mut T),
{
    data: T,
    clear_fn: F,
}

impl<T, F> TempSensitive<T, F>
where
    F: FnMut(&mut T),
{
    pub fn new(data: T, clear_fn: F) -> Self {
        Self { data, clear_fn }
    }

    pub fn get(&self) -> &T {
        &self.data
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

impl<T, F> Drop for TempSensitive<T, F>
where
    F: FnMut(&mut T),
{
    fn drop(&mut self) {
        (self.clear_fn)(&mut self.data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::errors::WalletError;

    #[test]
    fn test_secure_buffer() {
        let mut buffer = SecureBuffer::new(32).unwrap();
        assert_eq!(buffer.len(), 32);
        assert!(!buffer.is_empty());

        let data = b"Hello, Secure World!";
        buffer.write(data).unwrap();

        let mut read_data = vec![0u8; data.len()];
        buffer.read(&mut read_data).unwrap();
        assert_eq!(&read_data, data);
    }

    #[test]
    fn test_clear_sensitive() {
        let mut data = [1, 2, 3, 4, 5];
        clear_sensitive(&mut data);
        assert_eq!(data, [0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_secure_string() {
        let secret = "my_secret_password";
        let secure_str = SecureString::new(secret).unwrap();
        assert_eq!(secure_str.len(), secret.len());
        assert!(!secure_str.is_empty());

        let revealed = secure_str.reveal().unwrap();
        assert_eq!(revealed, secret);
    }

    #[test]
    fn test_temp_sensitive() {
        let mut cleared = false;
        {
            let mut temp = TempSensitive::new(42, |x| {
                *x = 0;
                cleared = true;
            });
            assert_eq!(*temp.get(), 42);
            *temp.get_mut() = 100;
            assert_eq!(*temp.get(), 100);
        }
        assert!(cleared);
    }

    #[test]
    fn test_secure_allocator() {
        let mut allocator = SecureAllocator::new();
        let result = allocator.alloc_locked(64);
        if let Ok(buffer) = result {
            assert_eq!(buffer.len(), 64);
            let _ = allocator.unlock_all();
        }
    }

    #[test]
    fn test_secure_buffer_new_zero_fails() {
        let res = SecureBuffer::new(0);
        assert!(matches!(res, Err(WalletError::InvalidInput(_))));
    }

    #[test]
    fn test_secure_buffer_write_too_large_fails() {
        let mut buffer = SecureBuffer::new(8).unwrap();
        let data = vec![1u8; 16];
        let res = buffer.write(&data);
        assert!(matches!(res, Err(WalletError::InvalidInput(_))));
    }

    #[test]
    fn test_secure_buffer_partial_read_smaller_dest() {
        let mut buffer = SecureBuffer::new(16).unwrap();
        let pattern: Vec<u8> = (0u8..16u8).collect();
        buffer.write(&pattern).unwrap();

        let mut dest = vec![0u8; 8];
        let read_len = buffer.read(&mut dest).unwrap();
        assert_eq!(read_len, 8);
        assert_eq!(dest, pattern[..8]);
    }

    #[test]
    fn test_secure_buffer_read_into_larger_dest() {
        let mut buffer = SecureBuffer::new(8).unwrap();
        let pattern: Vec<u8> = (1u8..=8u8).collect();
        buffer.write(&pattern).unwrap();

        let mut dest = vec![0u8; 16];
        let read_len = buffer.read(&mut dest).unwrap();
        assert_eq!(read_len, 8);
        assert_eq!(&dest[..8], &pattern[..]);
        assert!(dest[8..].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_clear_sensitive_data_overwrites_to_zero() {
        let mut data = vec![0xAA; 32];
        unsafe {
            clear_sensitive_data(data.as_mut_ptr(), data.len());
        }
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_clear_sensitive_data_zero_len_noop() {
        let mut data: Vec<u8> = (0u8..16u8).collect();
        let original = data.clone();
        unsafe {
            clear_sensitive_data(data.as_mut_ptr(), 0);
        }
        assert_eq!(data, original);
    }

    #[test]
    fn test_secure_string_empty_fails() {
        let res = SecureString::new("");
        assert!(matches!(res, Err(WalletError::InvalidInput(_))));
    }

    #[test]
    fn test_as_mut_slice_mutation() {
        let mut buffer = SecureBuffer::new(10).unwrap();
        let initial = vec![1u8; 10];
        buffer.write(&initial).unwrap();

        unsafe {
            let s = buffer.as_mut_slice();
            for (i, b) in s.iter_mut().enumerate() {
                *b = (i as u8) + 10;
            }
        }

        let expected: Vec<u8> = (0..10u8).map(|i| i + 10).collect();
        assert_eq!(buffer.as_slice(), &expected[..]);
    }

    #[test]
    fn test_lock_and_unlock_memory_smoke() {
        let mut data = vec![0u8; 64];
        let ptr = data.as_mut_ptr() as *const u8;
        let len = data.len();

        let lock_res = lock_memory(ptr, len);

        if cfg!(feature = "memlock") {
            match lock_res {
                Ok(()) => {
                    let unlock_res = unlock_memory(ptr, len);
                    assert!(unlock_res.is_ok(), "Unlocking should succeed if locking succeeded.");
                }
                Err(e) => {
                    tracing::debug!("Note: Memory locking failed with OS error: {}. This is often expected in test environments without special privileges.", e);
                }
            }
        } else {
            assert!(lock_res.is_ok(), "lock_memory should be a no-op and return Ok(())");
            let unlock_res = unlock_memory(ptr, len);
            assert!(unlock_res.is_ok(), "unlock_memory should be a no-op and return Ok(())");
        }
    }

    #[test]
    fn test_secure_allocator_unlock_all_idempotent() {
        let mut allocator = SecureAllocator::new();
        let _ = allocator.alloc_locked(32);
        let _ = allocator.unlock_all();
        let second = allocator.unlock_all();
        assert!(second.is_ok());
    }

    #[test]
    fn test_secure_buffer_clone_independent() {
        let mut original = SecureBuffer::new(8).unwrap();
        let a = vec![0x11u8; 8];
        original.write(&a).unwrap();

        let mut cloned = original.clone();
        let b = vec![0x22u8; 8];
        cloned.write(&b).unwrap();

        assert_eq!(original.as_slice(), &a[..]);
        assert_eq!(cloned.as_slice(), &b[..]);
    }

    #[test]
    fn test_secure_buffer_clone_contents_equal_initially() {
        let mut original = SecureBuffer::new(6).unwrap();
        let data = [9u8, 8, 7, 6, 5, 4];
        original.write(&data).unwrap();

        let cloned = original.clone();
        assert_eq!(original.as_slice(), cloned.as_slice());
    }
}
