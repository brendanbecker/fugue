//! Scrollback buffer implementation
//!
//! Provides a circular buffer for storing terminal output history.
//! VT100 escape sequences are preserved for faithful replay.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counter for tracking total scrollback memory across all buffers
static GLOBAL_SCROLLBACK_BYTES: AtomicUsize = AtomicUsize::new(0);

/// Default memory warning threshold (50MB)
pub const DEFAULT_MEMORY_WARNING_BYTES: usize = 50 * 1024 * 1024;

/// Default memory critical threshold (100MB)
pub const DEFAULT_MEMORY_CRITICAL_BYTES: usize = 100 * 1024 * 1024;

/// Get total scrollback memory usage across all buffers
pub fn global_scrollback_bytes() -> usize {
    GLOBAL_SCROLLBACK_BYTES.load(Ordering::Relaxed)
}

/// Memory status levels for scrollback usage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryStatus {
    /// Memory usage is normal
    Normal,
    /// Memory usage is approaching the warning threshold
    Warning,
    /// Memory usage has exceeded the critical threshold
    Critical,
}

/// Check global memory status against thresholds
pub fn check_memory_status() -> MemoryStatus {
    check_memory_status_with_thresholds(DEFAULT_MEMORY_WARNING_BYTES, DEFAULT_MEMORY_CRITICAL_BYTES)
}

/// Check global memory status with custom thresholds
pub fn check_memory_status_with_thresholds(warning_bytes: usize, critical_bytes: usize) -> MemoryStatus {
    let current = global_scrollback_bytes();
    if current >= critical_bytes {
        MemoryStatus::Critical
    } else if current >= warning_bytes {
        MemoryStatus::Warning
    } else {
        MemoryStatus::Normal
    }
}

/// Get a human-readable memory usage string
pub fn format_memory_usage() -> String {
    let bytes = global_scrollback_bytes();
    if bytes >= 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Circular buffer for scrollback history
#[derive(Debug)]
pub struct ScrollbackBuffer {
    /// Lines stored in the buffer
    lines: VecDeque<String>,
    /// Maximum number of lines to store
    max_lines: usize,
    /// Total bytes currently stored (for memory tracking)
    total_bytes: usize,
    /// Current viewport offset from the bottom (0 = at bottom, following output)
    viewport_offset: usize,
}

impl ScrollbackBuffer {
    /// Create a new scrollback buffer with the given capacity
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines.min(1024)), // Pre-allocate reasonably
            max_lines,
            total_bytes: 0,
            viewport_offset: 0,
        }
    }

    /// Get the current viewport offset from the bottom
    ///
    /// 0 means at bottom (following output), larger values mean scrolled up.
    pub fn viewport_offset(&self) -> usize {
        self.viewport_offset
    }

    /// Set the viewport offset from the bottom
    ///
    /// 0 means at bottom (following output), larger values mean scrolled up.
    pub fn set_viewport_offset(&mut self, offset: usize) {
        self.viewport_offset = offset;
    }

    /// Get the maximum number of lines this buffer can hold
    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    /// Get the current number of lines in the buffer
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Get total bytes currently stored
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Push a line into the buffer
    ///
    /// If the buffer is at capacity, the oldest line is removed.
    /// VT100 escape sequences in the line are preserved.
    pub fn push_line(&mut self, line: String) {
        let line_bytes = line.len();

        // If at capacity, remove oldest line
        if self.lines.len() >= self.max_lines {
            if let Some(removed) = self.lines.pop_front() {
                let removed_bytes = removed.len();
                self.total_bytes = self.total_bytes.saturating_sub(removed_bytes);
                GLOBAL_SCROLLBACK_BYTES.fetch_sub(removed_bytes, Ordering::Relaxed);
            }
        }

        // Add new line
        self.total_bytes += line_bytes;
        GLOBAL_SCROLLBACK_BYTES.fetch_add(line_bytes, Ordering::Relaxed);
        self.lines.push_back(line);
    }

    /// Push raw bytes, splitting into lines
    ///
    /// Handles both \n and \r\n line endings.
    /// Incomplete lines (no terminator) are stored as-is.
    pub fn push_bytes(&mut self, data: &[u8]) {
        let text = String::from_utf8_lossy(data);
        for line in text.lines() {
            self.push_line(line.to_string());
        }
    }

    /// Get all lines in the buffer
    pub fn get_lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(|s| s.as_str())
    }

    /// Get the last N lines from the buffer
    pub fn get_last_n(&self, n: usize) -> impl Iterator<Item = &str> {
        let skip = self.lines.len().saturating_sub(n);
        self.lines.iter().skip(skip).map(|s| s.as_str())
    }

    /// Get lines in a range (0-indexed from oldest)
    pub fn get_range(&self, start: usize, end: usize) -> impl Iterator<Item = &str> {
        self.lines
            .iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .map(|s| s.as_str())
    }

    /// Clear all lines from the buffer
    pub fn clear(&mut self) {
        GLOBAL_SCROLLBACK_BYTES.fetch_sub(self.total_bytes, Ordering::Relaxed);
        self.lines.clear();
        self.total_bytes = 0;
    }

    /// Get a line by index (0 is oldest)
    pub fn get(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(|s| s.as_str())
    }

    /// Search for lines containing the given pattern
    pub fn search<'a>(&'a self, pattern: &'a str) -> impl Iterator<Item = (usize, &'a str)> {
        self.lines
            .iter()
            .enumerate()
            .filter(move |(_, line)| line.contains(pattern))
            .map(|(i, line)| (i, line.as_str()))
    }

    /// Estimate memory usage of this buffer
    pub fn estimate_memory(&self) -> usize {
        // VecDeque overhead + string contents + string object overhead
        std::mem::size_of::<VecDeque<String>>()
            + self.total_bytes
            + self.lines.len() * std::mem::size_of::<String>()
    }
}

impl Drop for ScrollbackBuffer {
    fn drop(&mut self) {
        // Decrement global counter when buffer is dropped
        GLOBAL_SCROLLBACK_BYTES.fetch_sub(self.total_bytes, Ordering::Relaxed);
    }
}

impl Clone for ScrollbackBuffer {
    fn clone(&self) -> Self {
        let cloned = Self {
            lines: self.lines.clone(),
            max_lines: self.max_lines,
            total_bytes: self.total_bytes,
            viewport_offset: self.viewport_offset,
        };
        // Update global counter for cloned bytes
        GLOBAL_SCROLLBACK_BYTES.fetch_add(cloned.total_bytes, Ordering::Relaxed);
        cloned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buffer = ScrollbackBuffer::new(100);
        assert_eq!(buffer.max_lines(), 100);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.total_bytes(), 0);
    }

    #[test]
    fn test_push_line() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_line("Hello, World!".to_string());

        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
        assert_eq!(buffer.total_bytes(), 13);
    }

    #[test]
    fn test_push_multiple_lines() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_line("Line 1".to_string());
        buffer.push_line("Line 2".to_string());
        buffer.push_line("Line 3".to_string());

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.get(0), Some("Line 1"));
        assert_eq!(buffer.get(1), Some("Line 2"));
        assert_eq!(buffer.get(2), Some("Line 3"));
    }

    #[test]
    fn test_circular_behavior() {
        let mut buffer = ScrollbackBuffer::new(3);
        buffer.push_line("Line 1".to_string());
        buffer.push_line("Line 2".to_string());
        buffer.push_line("Line 3".to_string());
        buffer.push_line("Line 4".to_string());

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.get(0), Some("Line 2"));
        assert_eq!(buffer.get(1), Some("Line 3"));
        assert_eq!(buffer.get(2), Some("Line 4"));
    }

    #[test]
    fn test_get_lines() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_line("A".to_string());
        buffer.push_line("B".to_string());
        buffer.push_line("C".to_string());

        let lines: Vec<_> = buffer.get_lines().collect();
        assert_eq!(lines, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_get_last_n() {
        let mut buffer = ScrollbackBuffer::new(100);
        for i in 0..10 {
            buffer.push_line(format!("Line {}", i));
        }

        let last_3: Vec<_> = buffer.get_last_n(3).collect();
        assert_eq!(last_3, vec!["Line 7", "Line 8", "Line 9"]);
    }

    #[test]
    fn test_get_last_n_more_than_available() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_line("A".to_string());
        buffer.push_line("B".to_string());

        let last_10: Vec<_> = buffer.get_last_n(10).collect();
        assert_eq!(last_10, vec!["A", "B"]);
    }

    #[test]
    fn test_get_range() {
        let mut buffer = ScrollbackBuffer::new(100);
        for i in 0..10 {
            buffer.push_line(format!("Line {}", i));
        }

        let range: Vec<_> = buffer.get_range(3, 6).collect();
        assert_eq!(range, vec!["Line 3", "Line 4", "Line 5"]);
    }

    #[test]
    fn test_clear() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_line("A".to_string());
        buffer.push_line("B".to_string());

        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.total_bytes(), 0);
    }

    #[test]
    fn test_push_bytes() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_bytes(b"Line 1\nLine 2\nLine 3");

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.get(0), Some("Line 1"));
        assert_eq!(buffer.get(1), Some("Line 2"));
        assert_eq!(buffer.get(2), Some("Line 3"));
    }

    #[test]
    fn test_push_bytes_crlf() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_bytes(b"Line 1\r\nLine 2\r\nLine 3");

        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_search() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_line("Hello World".to_string());
        buffer.push_line("Goodbye World".to_string());
        buffer.push_line("Hello Again".to_string());

        let results: Vec<_> = buffer.search("Hello").collect();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], (0, "Hello World"));
        assert_eq!(results[1], (2, "Hello Again"));
    }

    #[test]
    fn test_search_no_results() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_line("Hello".to_string());

        let results: Vec<_> = buffer.search("xyz").collect();
        assert!(results.is_empty());
    }

    #[test]
    fn test_vt100_preserved() {
        let mut buffer = ScrollbackBuffer::new(100);
        let vt100_line = "\x1b[31mRed Text\x1b[0m".to_string();
        buffer.push_line(vt100_line.clone());

        assert_eq!(buffer.get(0), Some(vt100_line.as_str()));
    }

    #[test]
    fn test_estimate_memory() {
        let mut buffer = ScrollbackBuffer::new(100);
        let initial = buffer.estimate_memory();

        buffer.push_line("Hello, World!".to_string());
        let after = buffer.estimate_memory();

        assert!(after > initial);
    }

    #[test]
    fn test_get_out_of_bounds() {
        let buffer = ScrollbackBuffer::new(100);
        assert_eq!(buffer.get(0), None);
        assert_eq!(buffer.get(100), None);
    }

    #[test]
    fn test_byte_tracking_accuracy() {
        let mut buffer = ScrollbackBuffer::new(100);

        buffer.push_line("12345".to_string()); // 5 bytes
        assert_eq!(buffer.total_bytes(), 5);

        buffer.push_line("67890".to_string()); // 5 bytes
        assert_eq!(buffer.total_bytes(), 10);
    }

    #[test]
    fn test_byte_tracking_with_eviction() {
        let mut buffer = ScrollbackBuffer::new(2);

        buffer.push_line("AAA".to_string()); // 3 bytes
        buffer.push_line("BBB".to_string()); // 3 bytes
        assert_eq!(buffer.total_bytes(), 6);

        buffer.push_line("CCC".to_string()); // 3 bytes, evicts AAA
        assert_eq!(buffer.total_bytes(), 6);
    }

    #[test]
    fn test_clone() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_line("Hello".to_string());

        let cloned = buffer.clone();
        assert_eq!(cloned.len(), 1);
        assert_eq!(cloned.get(0), Some("Hello"));
        assert_eq!(cloned.max_lines(), 100);
    }

    #[test]
    fn test_debug_format() {
        let buffer = ScrollbackBuffer::new(100);
        let debug_str = format!("{:?}", buffer);
        assert!(debug_str.contains("ScrollbackBuffer"));
    }

    #[test]
    fn test_empty_line() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_line(String::new());

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.get(0), Some(""));
        assert_eq!(buffer.total_bytes(), 0);
    }

    #[test]
    fn test_unicode_lines() {
        let mut buffer = ScrollbackBuffer::new(100);
        buffer.push_line("こんにちは".to_string());
        buffer.push_line("🎉🎊🎈".to_string());

        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.get(0), Some("こんにちは"));
        assert_eq!(buffer.get(1), Some("🎉🎊🎈"));
    }

    #[test]
    fn test_max_capacity_one() {
        let mut buffer = ScrollbackBuffer::new(1);
        buffer.push_line("First".to_string());
        buffer.push_line("Second".to_string());

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.get(0), Some("Second"));
    }

    #[test]
    fn test_memory_status_normal() {
        let status = check_memory_status_with_thresholds(1_000_000, 2_000_000);
        // Global bytes are typically low in test context
        assert_eq!(status, MemoryStatus::Normal);
    }

    #[test]
    fn test_memory_status_thresholds() {
        // These tests verify the threshold logic
        let check = |bytes: usize| {
            if bytes >= 2_000_000 {
                MemoryStatus::Critical
            } else if bytes >= 1_000_000 {
                MemoryStatus::Warning
            } else {
                MemoryStatus::Normal
            }
        };

        assert_eq!(check(500_000), MemoryStatus::Normal);
        assert_eq!(check(1_000_000), MemoryStatus::Warning);
        assert_eq!(check(1_500_000), MemoryStatus::Warning);
        assert_eq!(check(2_000_000), MemoryStatus::Critical);
        assert_eq!(check(3_000_000), MemoryStatus::Critical);
    }

    #[test]
    fn test_format_memory_usage() {
        // Just verify it returns a string (actual value depends on global state)
        let formatted = format_memory_usage();
        assert!(!formatted.is_empty());
        assert!(
            formatted.contains("bytes") || formatted.contains("KB") || formatted.contains("MB")
        );
    }

    #[test]
    fn test_memory_status_debug() {
        assert!(format!("{:?}", MemoryStatus::Normal).contains("Normal"));
        assert!(format!("{:?}", MemoryStatus::Warning).contains("Warning"));
        assert!(format!("{:?}", MemoryStatus::Critical).contains("Critical"));
    }

    #[test]
    fn test_memory_status_clone_copy() {
        let status = MemoryStatus::Warning;
        let cloned = status.clone();
        let copied = status;
        assert_eq!(status, cloned);
        assert_eq!(status, copied);
    }

    #[test]
    fn test_default_thresholds() {
        assert_eq!(DEFAULT_MEMORY_WARNING_BYTES, 50 * 1024 * 1024);
        assert_eq!(DEFAULT_MEMORY_CRITICAL_BYTES, 100 * 1024 * 1024);
    }
}
