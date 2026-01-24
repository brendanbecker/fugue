//! Scrollback buffer persistence
//!
//! Handles capturing, compressing, and restoring scrollback buffer contents.
//! Supports multiple compression methods for efficient storage.

// Scaffolding for crash recovery feature - not all methods are wired up yet
#![allow(dead_code)]

use tracing::{debug, warn};

use fugue_utils::{CcmuxError, Result};

use super::types::{CompressionMethod, ScrollbackSnapshot};

/// Configuration for scrollback persistence
#[derive(Debug, Clone)]
pub struct ScrollbackConfig {
    /// Maximum lines to persist
    pub max_lines: usize,
    /// Compression method to use
    pub compression: CompressionMethod,
    /// Compression level for zstd (1-22, higher = better ratio)
    pub zstd_level: i32,
}

impl Default for ScrollbackConfig {
    fn default() -> Self {
        Self {
            max_lines: 1000,
            compression: CompressionMethod::Lz4,
            zstd_level: 3, // Fast compression
        }
    }
}

/// Captures and compresses scrollback buffer contents
pub struct ScrollbackCapture {
    config: ScrollbackConfig,
}

impl Default for ScrollbackCapture {
    fn default() -> Self {
        Self::new(ScrollbackConfig::default())
    }
}

impl ScrollbackCapture {
    /// Create a new scrollback capture with the given config
    pub fn new(config: ScrollbackConfig) -> Self {
        Self { config }
    }

    /// Capture scrollback content and create a snapshot
    ///
    /// Takes raw scrollback lines and compresses them for storage.
    pub fn capture(&self, lines: &[String]) -> Result<ScrollbackSnapshot> {
        // Limit to configured max lines
        let lines_to_capture = if lines.len() > self.config.max_lines {
            &lines[lines.len() - self.config.max_lines..]
        } else {
            lines
        };

        let line_count = lines_to_capture.len();

        if line_count == 0 {
            return Ok(ScrollbackSnapshot {
                line_count: 0,
                compressed_data: Vec::new(),
                compression: CompressionMethod::None,
            });
        }

        // Serialize lines to bytes (newline-separated)
        let raw_data: Vec<u8> = lines_to_capture.join("\n").into_bytes();

        // Compress based on method
        let (compressed_data, compression) = self.compress(&raw_data)?;

        debug!(
            "Captured {} lines ({} bytes) -> {} bytes ({:?})",
            line_count,
            raw_data.len(),
            compressed_data.len(),
            compression
        );

        Ok(ScrollbackSnapshot {
            line_count,
            compressed_data,
            compression,
        })
    }

    /// Capture scrollback from raw bytes (already newline-separated)
    pub fn capture_bytes(&self, data: &[u8], line_count: usize) -> Result<ScrollbackSnapshot> {
        if data.is_empty() {
            return Ok(ScrollbackSnapshot {
                line_count: 0,
                compressed_data: Vec::new(),
                compression: CompressionMethod::None,
            });
        }

        let (compressed_data, compression) = self.compress(data)?;

        debug!(
            "Captured {} lines ({} bytes) -> {} bytes ({:?})",
            line_count,
            data.len(),
            compressed_data.len(),
            compression
        );

        Ok(ScrollbackSnapshot {
            line_count,
            compressed_data,
            compression,
        })
    }

    /// Compress data using configured method
    fn compress(&self, data: &[u8]) -> Result<(Vec<u8>, CompressionMethod)> {
        match self.config.compression {
            CompressionMethod::None => Ok((data.to_vec(), CompressionMethod::None)),
            CompressionMethod::Lz4 => {
                let compressed = lz4_flex::compress_prepend_size(data);
                Ok((compressed, CompressionMethod::Lz4))
            }
            CompressionMethod::Zstd => {
                let compressed = zstd::encode_all(data, self.config.zstd_level).map_err(|e| {
                    CcmuxError::persistence(format!("Zstd compression failed: {}", e))
                })?;
                Ok((compressed, CompressionMethod::Zstd))
            }
        }
    }
}

/// Decompresses and restores scrollback content
pub struct ScrollbackRestore;

impl ScrollbackRestore {
    /// Restore scrollback content from a snapshot
    ///
    /// Returns the decompressed lines.
    pub fn restore(snapshot: &ScrollbackSnapshot) -> Result<Vec<String>> {
        if snapshot.line_count == 0 || snapshot.compressed_data.is_empty() {
            return Ok(Vec::new());
        }

        // Decompress
        let raw_data = Self::decompress(snapshot)?;

        // Parse lines
        let content = String::from_utf8(raw_data).map_err(|e| {
            CcmuxError::persistence(format!("Invalid UTF-8 in scrollback: {}", e))
        })?;

        let lines: Vec<String> = content.lines().map(String::from).collect();

        if lines.len() != snapshot.line_count {
            warn!(
                "Scrollback line count mismatch: expected {}, got {}",
                snapshot.line_count,
                lines.len()
            );
        }

        Ok(lines)
    }

    /// Restore scrollback as raw bytes
    pub fn restore_bytes(snapshot: &ScrollbackSnapshot) -> Result<Vec<u8>> {
        if snapshot.compressed_data.is_empty() {
            return Ok(Vec::new());
        }

        Self::decompress(snapshot)
    }

    /// Decompress data based on method
    fn decompress(snapshot: &ScrollbackSnapshot) -> Result<Vec<u8>> {
        match snapshot.compression {
            CompressionMethod::None => Ok(snapshot.compressed_data.clone()),
            CompressionMethod::Lz4 => {
                lz4_flex::decompress_size_prepended(&snapshot.compressed_data).map_err(|e| {
                    CcmuxError::persistence(format!("LZ4 decompression failed: {}", e))
                })
            }
            CompressionMethod::Zstd => {
                zstd::decode_all(snapshot.compressed_data.as_slice()).map_err(|e| {
                    CcmuxError::persistence(format!("Zstd decompression failed: {}", e))
                })
            }
        }
    }
}

/// Calculate approximate compression ratio
pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f64 {
    if compressed_size == 0 {
        return 0.0;
    }
    original_size as f64 / compressed_size as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrollback_config_default() {
        let config = ScrollbackConfig::default();
        assert_eq!(config.max_lines, 1000);
        assert_eq!(config.compression, CompressionMethod::Lz4);
    }

    #[test]
    fn test_capture_empty() {
        let capture = ScrollbackCapture::default();
        let snapshot = capture.capture(&[]).unwrap();

        assert_eq!(snapshot.line_count, 0);
        assert!(snapshot.compressed_data.is_empty());
    }

    #[test]
    fn test_capture_single_line() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::None,
            ..Default::default()
        });

        let lines = vec!["Hello, world!".to_string()];
        let snapshot = capture.capture(&lines).unwrap();

        assert_eq!(snapshot.line_count, 1);
        assert_eq!(snapshot.compression, CompressionMethod::None);
    }

    #[test]
    fn test_capture_multiple_lines() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::None,
            ..Default::default()
        });

        let lines: Vec<String> = (0..10).map(|i| format!("Line {}", i)).collect();
        let snapshot = capture.capture(&lines).unwrap();

        assert_eq!(snapshot.line_count, 10);
    }

    #[test]
    fn test_capture_truncates_to_max_lines() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            max_lines: 5,
            compression: CompressionMethod::None,
            ..Default::default()
        });

        let lines: Vec<String> = (0..10).map(|i| format!("Line {}", i)).collect();
        let snapshot = capture.capture(&lines).unwrap();

        assert_eq!(snapshot.line_count, 5);
    }

    #[test]
    fn test_restore_empty() {
        let snapshot = ScrollbackSnapshot {
            line_count: 0,
            compressed_data: Vec::new(),
            compression: CompressionMethod::None,
        };

        let lines = ScrollbackRestore::restore(&snapshot).unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn test_roundtrip_no_compression() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::None,
            ..Default::default()
        });

        let original: Vec<String> = (0..100).map(|i| format!("Line {}: some content", i)).collect();
        let snapshot = capture.capture(&original).unwrap();
        let restored = ScrollbackRestore::restore(&snapshot).unwrap();

        assert_eq!(original, restored);
    }

    #[test]
    fn test_roundtrip_lz4_compression() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::Lz4,
            ..Default::default()
        });

        let original: Vec<String> = (0..100).map(|i| format!("Line {}: some repeated content", i)).collect();
        let snapshot = capture.capture(&original).unwrap();

        assert_eq!(snapshot.compression, CompressionMethod::Lz4);

        let restored = ScrollbackRestore::restore(&snapshot).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_roundtrip_zstd_compression() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::Zstd,
            zstd_level: 1,
            ..Default::default()
        });

        let original: Vec<String> = (0..100).map(|i| format!("Line {}: some repeated content", i)).collect();
        let snapshot = capture.capture(&original).unwrap();

        assert_eq!(snapshot.compression, CompressionMethod::Zstd);

        let restored = ScrollbackRestore::restore(&snapshot).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_compression_ratio_calculation() {
        assert_eq!(compression_ratio(1000, 500), 2.0);
        assert_eq!(compression_ratio(1000, 1000), 1.0);
        assert_eq!(compression_ratio(1000, 0), 0.0);
    }

    #[test]
    fn test_lz4_provides_compression() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::Lz4,
            ..Default::default()
        });

        // Highly compressible content
        let original: Vec<String> = (0..1000)
            .map(|_| "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string())
            .collect();

        let snapshot = capture.capture(&original).unwrap();
        let original_size: usize = original.iter().map(|l| l.len() + 1).sum();

        // LZ4 should compress this significantly
        assert!(
            snapshot.compressed_data.len() < original_size / 2,
            "Expected significant compression, got {} -> {}",
            original_size,
            snapshot.compressed_data.len()
        );
    }

    #[test]
    fn test_zstd_provides_compression() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::Zstd,
            zstd_level: 3,
            ..Default::default()
        });

        // Highly compressible content
        let original: Vec<String> = (0..1000)
            .map(|_| "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string())
            .collect();

        let snapshot = capture.capture(&original).unwrap();
        let original_size: usize = original.iter().map(|l| l.len() + 1).sum();

        // Zstd should compress this significantly
        assert!(
            snapshot.compressed_data.len() < original_size / 2,
            "Expected significant compression, got {} -> {}",
            original_size,
            snapshot.compressed_data.len()
        );
    }

    #[test]
    fn test_capture_bytes() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::None,
            ..Default::default()
        });

        let data = b"Line 1\nLine 2\nLine 3";
        let snapshot = capture.capture_bytes(data, 3).unwrap();

        assert_eq!(snapshot.line_count, 3);
        assert_eq!(snapshot.compressed_data, data.to_vec());
    }

    #[test]
    fn test_restore_bytes() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::Lz4,
            ..Default::default()
        });

        let data = b"Line 1\nLine 2\nLine 3";
        let snapshot = capture.capture_bytes(data, 3).unwrap();
        let restored = ScrollbackRestore::restore_bytes(&snapshot).unwrap();

        assert_eq!(restored, data.to_vec());
    }

    #[test]
    fn test_special_characters() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::Lz4,
            ..Default::default()
        });

        let original = vec![
            "Tab:\there".to_string(),
            "Unicode: \u{1F600} emoji".to_string(),
            "Escape: \x1b[32mgreen\x1b[0m".to_string(),
        ];

        let snapshot = capture.capture(&original).unwrap();
        let restored = ScrollbackRestore::restore(&snapshot).unwrap();

        assert_eq!(original, restored);
    }

    #[test]
    fn test_empty_lines() {
        let capture = ScrollbackCapture::new(ScrollbackConfig {
            compression: CompressionMethod::None,
            ..Default::default()
        });

        // Note: trailing empty lines are not preserved due to how
        // lines() iterator works. This is acceptable for scrollback.
        let original = vec![
            "Line 1".to_string(),
            "".to_string(),
            "Line 3".to_string(),
        ];

        let snapshot = capture.capture(&original).unwrap();
        let restored = ScrollbackRestore::restore(&snapshot).unwrap();

        assert_eq!(original, restored);
    }
}
