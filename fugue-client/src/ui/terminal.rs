//! Terminal initialization and cleanup
//!
//! Provides safe terminal mode management using crossterm backend.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use std::io::{self, Stdout};

use crossterm::{
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;

use fugue_utils::Result;

/// Terminal wrapper that handles initialization and cleanup
pub struct Terminal {
    terminal: ratatui::Terminal<CrosstermBackend<Stdout>>,
}

impl Terminal {
    /// Initialize the terminal in raw mode with alternate screen
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;

        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste
        )?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = ratatui::Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    /// Get mutable reference to the underlying terminal for drawing
    pub fn terminal_mut(&mut self) -> &mut ratatui::Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    /// Get terminal size (columns, rows)
    pub fn size(&self) -> Result<(u16, u16)> {
        let size = self.terminal.size()?;
        Ok((size.width, size.height))
    }

    /// Force a full redraw on next frame
    pub fn clear(&mut self) -> Result<()> {
        self.terminal.clear()?;
        Ok(())
    }

    /// Restore terminal to original state
    fn restore() -> Result<()> {
        disable_raw_mode()?;
        execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableBracketedPaste
        )?;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Err(e) = Self::restore() {
            tracing::error!("Failed to restore terminal: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_restore_is_safe() {
        // Ensure restore doesn't panic even when not in raw mode
        // (This is a no-op test since we can't actually test terminal modes in CI)
        let _ = Terminal::restore();
    }
}
