use serde::{Deserialize, Serialize};

/// A wrapper for serde_json::Value that serializes as a JSON string for bincode compatibility.
///
/// Bincode doesn't support `deserialize_any` which `serde_json::Value` requires.
/// This wrapper serializes the JSON value as a string, which bincode can handle.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonValue(pub serde_json::Value);

impl JsonValue {
    /// Create a new JsonValue from a serde_json::Value
    pub fn new(value: serde_json::Value) -> Self {
        Self(value)
    }

    /// Get a reference to the inner value
    pub fn inner(&self) -> &serde_json::Value {
        &self.0
    }

    /// Consume the wrapper and return the inner value
    pub fn into_inner(self) -> serde_json::Value {
        self.0
    }
}

impl From<serde_json::Value> for JsonValue {
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl From<JsonValue> for serde_json::Value {
    fn from(value: JsonValue) -> Self {
        value.0
    }
}

impl std::ops::Deref for JsonValue {
    type Target = serde_json::Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for JsonValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as a JSON string for bincode compatibility
        let json_string = serde_json::to_string(&self.0).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&json_string)
    }
}

impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize from a JSON string
        let json_string = String::deserialize(deserializer)?;
        let value: serde_json::Value =
            serde_json::from_str(&json_string).map_err(serde::de::Error::custom)?;
        Ok(Self(value))
    }
}

/// Split direction for creating panes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Terminal dimensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dimensions {
    pub cols: u16,
    pub rows: u16,
}

impl Dimensions {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }
}

/// Actions that can be performed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Change focus/selection
    Focus,
    /// Send text input
    Input,
    /// Mutate layout (resize, split)
    Layout,
    /// Destructive actions (kill)
    Kill,
}

/// Type of client (FEAT-079)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientType {
    /// Interactive Terminal UI
    Tui,
    /// Automated Agent (MCP)
    Mcp,
    /// Command-line tool / Compat
    Compat,
    /// Unknown or legacy client
    Unknown,
}

/// Priority for mailbox messages (FEAT-073)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MailPriority {
    Info,
    Warning,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== SplitDirection Tests ====================

    #[test]
    fn test_split_direction_horizontal() {
        let dir = SplitDirection::Horizontal;
        assert_eq!(dir, SplitDirection::Horizontal);
        assert_ne!(dir, SplitDirection::Vertical);
    }

    #[test]
    fn test_split_direction_vertical() {
        let dir = SplitDirection::Vertical;
        assert_eq!(dir, SplitDirection::Vertical);
        assert_ne!(dir, SplitDirection::Horizontal);
    }

    #[test]
    fn test_split_direction_clone() {
        let dir = SplitDirection::Horizontal;
        let cloned = dir.clone();
        assert_eq!(dir, cloned);
    }

    #[test]
    fn test_split_direction_copy() {
        let dir = SplitDirection::Vertical;
        let copied = dir; // Copy semantics
        assert_eq!(dir, copied);
    }

    #[test]
    fn test_split_direction_debug() {
        assert_eq!(format!("{:?}", SplitDirection::Horizontal), "Horizontal");
        assert_eq!(format!("{:?}", SplitDirection::Vertical), "Vertical");
    }

    #[test]
    fn test_split_direction_serde() {
        let dir = SplitDirection::Horizontal;
        let serialized = bincode::serialize(&dir).unwrap();
        let deserialized: SplitDirection = bincode::deserialize(&serialized).unwrap();
        assert_eq!(dir, deserialized);
    }

    // ==================== Dimensions Tests ====================

    #[test]
    fn test_dimensions_new() {
        let dims = Dimensions::new(80, 24);
        assert_eq!(dims.cols, 80);
        assert_eq!(dims.rows, 24);
    }

    #[test]
    fn test_dimensions_equality() {
        let dims1 = Dimensions::new(80, 24);
        let dims2 = Dimensions::new(80, 24);
        let dims3 = Dimensions::new(120, 40);

        assert_eq!(dims1, dims2);
        assert_ne!(dims1, dims3);
    }

    #[test]
    fn test_dimensions_clone_copy() {
        let dims = Dimensions::new(100, 50);
        let cloned = dims.clone();
        let copied = dims; // Copy

        assert_eq!(dims, cloned);
        assert_eq!(dims, copied);
    }

    #[test]
    fn test_dimensions_debug() {
        let dims = Dimensions::new(80, 24);
        let debug = format!("{:?}", dims);
        assert!(debug.contains("80"));
        assert!(debug.contains("24"));
    }

    #[test]
    fn test_dimensions_zero() {
        let dims = Dimensions::new(0, 0);
        assert_eq!(dims.cols, 0);
        assert_eq!(dims.rows, 0);
    }

    #[test]
    fn test_dimensions_max_values() {
        let dims = Dimensions::new(u16::MAX, u16::MAX);
        assert_eq!(dims.cols, u16::MAX);
        assert_eq!(dims.rows, u16::MAX);
    }

    #[test]
    fn test_dimensions_serde() {
        let dims = Dimensions::new(80, 24);
        let serialized = bincode::serialize(&dims).unwrap();
        let deserialized: Dimensions = bincode::deserialize(&serialized).unwrap();
        assert_eq!(dims, deserialized);
    }
}
