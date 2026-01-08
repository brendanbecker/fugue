//! PTY handle wrapper for portable-pty

use std::io::{Read, Write};
use std::sync::Arc;

use ccmux_utils::{CcmuxError, Result};
use parking_lot::Mutex;
use portable_pty::{Child, MasterPty, PtySize};

/// Handle to a running PTY
pub struct PtyHandle {
    /// The master side of the PTY
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    /// The child process
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    /// Reader for PTY output
    reader: Arc<Mutex<Box<dyn Read + Send>>>,
    /// Writer for PTY input
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl PtyHandle {
    /// Create a new PTY handle from portable-pty components
    pub(crate) fn new(
        master: Box<dyn MasterPty + Send>,
        child: Box<dyn Child + Send + Sync>,
        reader: Box<dyn Read + Send>,
        writer: Box<dyn Write + Send>,
    ) -> Self {
        Self {
            master: Arc::new(Mutex::new(master)),
            child: Arc::new(Mutex::new(child)),
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        }
    }

    /// Write data to the PTY (sends to the child process)
    pub fn write(&self, data: &[u8]) -> Result<usize> {
        let mut writer = self.writer.lock();
        writer
            .write(data)
            .map_err(|e| CcmuxError::pty(format!("Write failed: {}", e)))
    }

    /// Write all data to the PTY
    pub fn write_all(&self, data: &[u8]) -> Result<()> {
        let mut writer = self.writer.lock();
        writer
            .write_all(data)
            .map_err(|e| CcmuxError::pty(format!("Write failed: {}", e)))
    }

    /// Read data from the PTY (output from the child process)
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        let mut reader = self.reader.lock();
        reader
            .read(buf)
            .map_err(|e| CcmuxError::pty(format!("Read failed: {}", e)))
    }

    /// Resize the PTY
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let master = self.master.lock();
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| CcmuxError::pty(format!("Resize failed: {}", e)))
    }

    /// Check if the child process has exited
    pub fn try_wait(&self) -> Result<Option<i32>> {
        let mut child = self.child.lock();
        match child.try_wait() {
            Ok(Some(status)) => Ok(Some(status.exit_code() as i32)),
            Ok(None) => Ok(None),
            Err(e) => Err(CcmuxError::pty(format!("Wait failed: {}", e))),
        }
    }

    /// Wait for the child process to exit
    pub fn wait(&self) -> Result<i32> {
        let mut child = self.child.lock();
        match child.wait() {
            Ok(status) => Ok(status.exit_code() as i32),
            Err(e) => Err(CcmuxError::pty(format!("Wait failed: {}", e))),
        }
    }

    /// Kill the child process
    pub fn kill(&self) -> Result<()> {
        let mut child = self.child.lock();
        child
            .kill()
            .map_err(|e| CcmuxError::pty(format!("Kill failed: {}", e)))
    }

    /// Get a clone of the reader (for async reading)
    pub fn clone_reader(&self) -> Arc<Mutex<Box<dyn Read + Send>>> {
        self.reader.clone()
    }

    /// Get a clone of the writer (for async writing)
    pub fn clone_writer(&self) -> Arc<Mutex<Box<dyn Write + Send>>> {
        self.writer.clone()
    }
}

impl std::fmt::Debug for PtyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtyHandle").finish_non_exhaustive()
    }
}
