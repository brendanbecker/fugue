//! Event replay buffer for client resync
//!
//! Maintains a ring buffer of recent server messages keyed by sequence number.
//! Used to catch up clients that have disconnected briefly.

// Scaffolding for crash recovery feature - not all methods are wired up yet
#![allow(dead_code)]

use std::collections::VecDeque;
use ccmux_protocol::ServerMessage;

/// Buffer for retaining recent events for replay
#[derive(Debug)]
pub struct ReplayBuffer {
    /// Events stored as (sequence_number, message)
    events: VecDeque<(u64, ServerMessage)>,
    /// Maximum number of events to keep
    max_events: usize,
    /// Minimum sequence number currently in buffer (inclusive)
    min_seq: u64,
    /// Maximum sequence number currently in buffer (inclusive)
    max_seq: u64,
}

impl ReplayBuffer {
    /// Create a new replay buffer with the given capacity
    pub fn new(max_events: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(max_events),
            max_events,
            min_seq: 0,
            max_seq: 0,
        }
    }

    /// Add an event to the buffer
    pub fn push(&mut self, seq: u64, event: ServerMessage) {
        // Enforce capacity
        if self.events.len() >= self.max_events {
            self.events.pop_front();
            // Update min_seq after pop
            if let Some((s, _)) = self.events.front() {
                self.min_seq = *s;
            }
        }

        // If buffer was empty, set min_seq
        if self.events.is_empty() {
            self.min_seq = seq;
        }

        self.events.push_back((seq, event));
        self.max_seq = seq;
    }

    /// Get the minimum sequence number in the buffer
    pub fn min_seq(&self) -> u64 {
        self.min_seq
    }

    /// Get the maximum sequence number in the buffer
    pub fn max_seq(&self) -> u64 {
        self.max_seq
    }

    /// Get events starting from the given sequence (exclusive)
    ///
    /// Returns `None` if the requested sequence is too old (gap > buffer).
    pub fn get_events_since(&self, since_seq: u64) -> Option<Vec<(u64, ServerMessage)>> {
        // If buffer is empty, we have no events. 
        // If since_seq == 0, we can return empty vec (no events yet).
        // If since_seq > 0, and we have no events, it implies we might have lost them?
        // But if max_seq == 0, effectively no events ever happened (or buffer cleared).
        if self.events.is_empty() {
            if since_seq == 0 {
                return Some(Vec::new());
            } else {
                // If the client asks for seq 100, and we have nothing, 
                // we assume we can't fulfill it? 
                // Actually if we have nothing, we can't help.
                // But if the server restarted, max_seq resets?
                // No, seq persists.
                // If buffer is empty but we have history on disk, we might return None to force snapshot.
                return None;
            }
        }

        // If requested sequence is older than our oldest event, we can't do full replay
        // We need the next event (since_seq + 1) to be present in the buffer.
        // So if min_seq > since_seq + 1, we have a gap.
        if self.min_seq > since_seq + 1 {
             return None;
        }

        // Collect events
        let mut result = Vec::new();
        for (seq, event) in &self.events {
            if *seq > since_seq {
                result.push((*seq, event.clone()));
            }
        }
        Some(result)
    }

    /// Get current sequence range
    pub fn range(&self) -> (u64, u64) {
        (self.min_seq, self.max_seq)
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.events.clear();
        self.min_seq = 0;
        self.max_seq = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccmux_protocol::ServerMessage;

    #[test]
    fn test_replay_buffer_capacity() {
        let mut buffer = ReplayBuffer::new(3);

        for i in 1..=5 {
            buffer.push(i, ServerMessage::Pong);
        }

        assert_eq!(buffer.events.len(), 3);
        assert_eq!(buffer.min_seq, 3);
        assert_eq!(buffer.max_seq, 5);
    }

    #[test]
    fn test_get_events_since() {
        let mut buffer = ReplayBuffer::new(10);
        
        for i in 1..=5 {
            buffer.push(i, ServerMessage::Pong);
        }

        // Case 1: Get all
        let events = buffer.get_events_since(0).unwrap();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].0, 1);

        // Case 2: Get some
        let events = buffer.get_events_since(3).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, 4);

        // Case 3: Get none (up to date)
        let events = buffer.get_events_since(5).unwrap();
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_gap_detection() {
        let mut buffer = ReplayBuffer::new(3);
        
        for i in 1..=10 {
            buffer.push(i, ServerMessage::Pong);
        }
        // Buffer should contain 8, 9, 10 (min_seq=8)

        // Asking for 6 (needs 7, but we start at 8) -> Gap
        assert!(buffer.get_events_since(6).is_none());

        // Asking for 7 (needs 8, we have 8) -> OK
        let events = buffer.get_events_since(7).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].0, 8);

        // Asking for 8 -> OK
        let events = buffer.get_events_since(8).unwrap();
        assert_eq!(events.len(), 2);
    }
}
