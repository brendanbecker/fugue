//! Default configuration values
//!
//! These are embedded in the binary and used when no config file exists.

/// Default configuration as TOML (for reference/documentation)
#[allow(dead_code)]
pub const DEFAULT_CONFIG_TOML: &str = r##"
# fugue configuration

[general]
# default_shell = "/bin/bash"
max_depth = 5
prefix_key = "Ctrl-a"

[appearance]
theme = "default"
status_position = "bottom"
border_style = "rounded"
show_pane_titles = true

[colors]
status_bg = "#282c34"
status_fg = "#abb2bf"
active_border = "#61afef"
inactive_border = "#5c6370"
claude_thinking = "#e5c07b"
claude_idle = "#98c379"
claude_error = "#e06c75"

[terminal]
scrollback_lines = 10000
render_interval_ms = 16
parser_timeout_secs = 5

[claude]
detection_enabled = true
detection_method = "pty"
show_status = true
auto_resume = true

[persistence]
checkpoint_interval_secs = 30
max_wal_size_mb = 128
screen_snapshot_lines = 500
"##;
