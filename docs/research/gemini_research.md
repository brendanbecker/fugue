# **Architectural Blueprint for ccmux: A Semantic Terminal Multiplexer for Claude Code**

## **1\. Terminal Emulation in Rust**

The foundational requirement for **ccmux** is the ability to emulate a terminal with high fidelity. Unlike traditional multiplexers that merely shovel bytes, ccmux must act as a fully realized terminal emulator in memory to interpret the semantic state of the hosted Claude Code process. This necessitates a deep integration of Pseudo-Terminal (PTY) management, ANSI state parsing, and rendering.

### **1.1 PTY Spawning and Management**

The Pseudo-Terminal (PTY) is the operating system primitive that allows a user-space program (ccmux) to act as a hardware terminal for another program (the shell or Claude Code). The current state of the art in the Rust ecosystem for PTY management is the portable-pty crate.

#### **1.1.1 State of the Art: portable-pty**

Originating from the WezTerm project, portable-pty provides a cross-platform abstraction over the disparate PTY interfaces of Unix (openpty, posix\_openpt) and Windows (ConPTY).1 Its architecture uses a trait-based system (PtySystem) to select the appropriate backend at runtime, isolating the consuming application from OS-specific implementation details.3

The library handles the complex "dance" required to spawn a PTY:

1. **Opening the Master:** The multiplexer requests a new PTY master file descriptor (FD).  
2. **Granting and Unlocking:** On Unix, specific ioctl calls (grantpt, unlockpt) are required to configure permissions for the slave end.  
3. **Spawning the Child:** The child process is forked. Within the child, setsid() is called to create a new session, and the slave FD is duplicated (dup2) onto standard input, output, and error (0, 1, 2).4  
4. **Handshake:** The master and slave pairs are returned to the parent process.

#### **1.1.2 Bidirectional I/O and Resizing**

In a multiplexer, I/O must be non-blocking. portable-pty provides master.take\_writer() and master.try\_clone\_reader(), yielding synchronous readers/writers. Integrating this with an async runtime like Tokio requires wrapping these blocking handles. On Unix, this is efficiently achieved by converting the raw file descriptors into tokio::fs::File or tokio::io::unix::AsyncFd, allowing the reactor to poll for readability rather than blocking a thread.6

Resizing (SIGWINCH):  
A critical responsibility of the PTY master is propagating window size changes. When the ccmux UI resizes, the internal PTY must be notified so the child application can reflow its text. portable-pty exposes a resize method on the MasterPty trait. Calling this updates the kernel's winsize structure and sends a SIGWINCH signal to the foreground process group of the PTY.2 Failure to propagate this correctly results in "torn" UIs where the application renders for a different geometry than is displayed.

#### **1.1.3 Implementation Strategy for ccmux**

The recommended approach is to encapsulate the PTY interaction in an actor (see Section 7). This actor holds the PtyPair and spawns two Tokio tasks: one to pump bytes from the PTY master to the alacritty\_terminal parser, and another to pump user input from the multiplexer client to the PTY master.

Rust

use portable\_pty::{native\_pty\_system, PtySize, CommandBuilder};  
use tokio::task;  
use anyhow::Result;

pub struct PtyActor {  
    // fields...  
}

impl PtyActor {  
    pub fn spawn\_shell() \-\> Result\<()\> {  
        let pty\_system \= native\_pty\_system();  
        let pair \= pty\_system.openpty(PtySize {  
            rows: 24, cols: 80, pixel\_width: 0, pixel\_height: 0,  
        })?;

        let cmd \= CommandBuilder::new("claude");  
        let \_child \= pair.slave.spawn\_command(cmd)?;

        // Move blocking reader to a blocking task or wrap with AsyncFd  
        let mut reader \= pair.master.try\_clone\_reader()?;  
        let mut writer \= pair.master.take\_writer()?;

        // Writer task (Input from user \-\> PTY)  
        // Reader task (PTY \-\> Parser)  
          
        Ok(())  
    }  
}

### **1.2 Terminal State Parsing**

The stream of bytes coming from the PTY contains ANSI escape codes that modify the terminal state (cursor position, colors, scrolling). To render this, ccmux needs a state machine that consumes these bytes and maintains a grid of cells.

#### **1.2.1 alacritty\_terminal vs. vt100**

The research identifies two primary candidates: alacritty\_terminal and vt100.

| Feature | alacritty\_terminal | vt100 |
| :---- | :---- | :---- |
| **Origin** | Extracted from the high-performance Alacritty emulator.8 | Independent library for in-memory emulation.10 |
| **Performance** | Extremely high throughput, optimized for OpenGL pipelines. | Moderate; efficient but lacks the micro-optimizations of Alacritty. |
| **Correctness** | Battle-tested against edge cases (vttest compliance). | Good compliance, but fewer active contributors/eyes. |
| **State Model** | Grid with circular buffer for scrollback. | Parser \+ Screen struct. |
| **Dependencies** | Heavier dependency tree (cross-platform handling). | Lightweight. |

Tradeoff Analysis:  
While vt100 is simpler to integrate, alacritty\_terminal offers superior handling of complex terminal states, such as the "Alternate Screen Buffer" used by TUI applications (like vim or Claude Code's internal pager).10 Alacritty's Grid implementation is specifically optimized for sparse updates and iterator performance, which is crucial for maintaining 60fps when rendering multiple panes.12  
**Recommendation:** Use alacritty\_terminal. The complexity of integration is outweighed by the correctness guarantees, particularly for a multiplexer that claims to be "Claude Aware" and may need to parse subtle visual cues in the output stream.

#### **1.2.2 Data Structures and Storage**

alacritty\_terminal represents the screen as a Grid\<Cell\>. The Grid handles the distinction between the writable region (viewport) and the history (scrollback).

* **Cursor:** Tracked separately. Attributes include line, column, and style.  
* **Alternate Screen:** The Term struct in alacritty\_terminal manages switching between the primary and alternate buffers via DECSET 1047/1049 sequences.9 ccmux must expose API hooks to query *which* buffer is active, as this affects scrolling behavior (scrollback is usually disabled in alternate screen mode).

### **1.3 Rendering Through Ratatui**

Ratatui (formerly tui-rs) is the presentation layer. It operates on an immediate-mode rendering principle using a Buffer of cells.14 The engineering challenge is bridging the alacritty\_terminal state to the ratatui buffer efficiently.

#### **1.3.1 Adapter Pattern Implementation**

There is no direct compatibility between Alacritty's Cell and Ratatui's Cell. An adapter widget is required.

**Rendering Pipeline:**

1. **Locking:** Acquire a read lock on the alacritty\_terminal::Term instance.  
2. **Iterator:** Use the renderable\_cells() iterator provided by Alacritty. This iterator yields only the cells that have content, skipping empty space, which is a significant performance optimization.12  
3. **Translation:** Convert Alacritty's Color and Flags (bold, italic, etc.) to Ratatui's Style.  
   * *Note:* Alacritty supports TrueColor (24-bit RGB). Ratatui also supports this; the mapping must preserve RGB values to ensure syntax highlighting in Claude Code looks correct.17  
4. **Writing:** Place the styled characters into the Ratatui Buffer at the calculated coordinates.

Performance Model:  
To maintain 60fps with 10+ panes, ccmux must avoid full-screen copies if possible. Ratatui's internal diffing algorithm (writing only changed cells to the actual terminal) handles the final optimization. However, the copy from Alacritty to Ratatui's back-buffer is CPU-bound.

* *Optimization:* Implement "dirty rect" tracking. If a PTY hasn't received new bytes since the last frame, skip the copy step for that pane entirely.

### **1.4 Terminal Capabilities**

For Claude Code to function correctly, ccmux must advertise specific capabilities via the TERM environment variable and terminfo.

* **TERM:** Set to xterm-256color or alacritty to ensure applications output standard sequences.  
* **True Color:** Set COLORTERM=truecolor.  
* **Mouse Support:** If ccmux supports mouse interaction (clicking to focus panes), it must also forward mouse events to the active pane if that application requests them (e.g., clicking inside htop). This involves parsing SGR mouse sequences from the host terminal and re-encoding them to the PTY master.19

## ---

**2\. Claude Code Internals**

The "Claude Awareness" of ccmux relies on inspecting the output stream and state of the Claude Code process.

### **2.1 State Detection via Visual Telemetry**

Research into Claude Code's CLI behavior reveals distinct visual patterns corresponding to its internal cognitive states.20 These are emitted to stderr or stdout using ANSI escape sequences to overwrite the current line (spinners).

**Identified States:**

* **Thinking/Channelling:** "Channelling..." (Confidence 0.8–1.0). Indicates retrieval.  
* **Synthesizing:** "Synthesizing..." (Confidence 0.6–0.9). Indicates reasoning/bridging.  
* **Discombobulating:** "Discombobulating..." (Confidence 0.2–0.6). High entropy/uncertainty.  
* **Waiting:** Static prompt awaiting user input.

Detection Algorithm:  
ccmux effectively acts as a stream processor. Before bytes are passed to the terminal emulator, they pass through a "State Detector":

1. **Buffer:** Maintain a small look-back buffer of the raw byte stream.  
2. **Pattern Match:** Scan for \\r (carriage return) followed by specific keywords. The use of \\r is the tell-tale sign of a spinner or status update that is meant to be transient.21  
3. **State Transition:** Upon matching "Synthesizing...", transition the internal PaneState to PaneState::Thinking(Synthesizing). This state can then be rendered in the ccmux status bar (e.g., changing the pane border color to yellow or pulsing).

Pitfall: Spinner Corruption  
A documented issue with Claude Code is that spinner animations can corrupt scrollback history if the terminal width causes line wrapping.21 ccmux can solve this by intercepting the spinner lines. Instead of writing them to the alacritty\_terminal grid (where they might pollute history), ccmux can strip them from the stream and render the spinner status natively in the multiplexer UI frame.

### **2.2 Output Structure**

While visual scraping is necessary for legacy/default modes, Claude Code supports structured output flags that are far more robust for programmatic control.

* \--output-format json: Emits a single JSON object at the end. Useful for non-interactive tasks but poor for real-time multiplexing.23  
* \--output-format stream-json: Emits newline-delimited JSON events.23 This is the critical integration point.

Recommended Integration:  
When ccmux spawns a "Managed Claude Pane," it should invoke Claude with \--output-format stream-json.

* **Data Channel:** The stdout stream contains JSON objects. ccmux parses these to extract the text content (for display) and metadata (token usage, cost, current tool use).  
* **Rendering:** ccmux renders the text content into the terminal grid but uses the metadata to populate a rich sidebar or status line, showing real-time costs and cognitive state without screen clutter.

### **2.3 Session Resume**

Claude Code's session management is key to crash recovery.

* **Mechanism:** claude \--resume \<session\_id\>.25  
* **Storage:** Session state is serialized in \~/.claude/ (likely LevelDB or JSON blobs).26  
* **Strategy:** ccmux must persist the mapping between its own PaneID and Claude's SessionID. If ccmux crashes and restarts, it can inspect this persistent map and automatically respawn Claude processes with \--resume \<id\>, effectively restoring the cognitive context even if the visual scrollback is lost.

### **2.4 Programmatic Interaction**

There is no formal IPC socket for running Claude instances. Interaction must occur via:

1. **STDIN Injection:** Writing text to the PTY master.  
2. **Filesystem Watchers:** Monitoring CLAUDE.md or specific trigger files, though this is high-latency.  
3. **MCP (Future):** See Section 6\.

## ---

**3\. Crash Recovery for Terminal Multiplexers**

Reliability is the defining feature of a multiplexer. The user's session must survive if the UI crashes.

### **3.1 Architecture: The Daemon-Client Separation**

To achieve persistence, ccmux cannot be a monolithic process. It must adopt the client-server architecture used by tmux and zellij.

* **The Server (Daemon):** This process runs in the background. It owns the PtyPair handles and the alacritty\_terminal::Grid state. It has *no* UI dependencies. It runs as a systemd user service or a detached daemon.  
* **The Client (UI):** This is the interface the user sees. It connects to the Daemon via a Unix Domain Socket (UDS). It sends input events (keypresses) and receives render instructions (diffs of the grid).

If the Client panics or is killed, the Daemon (and the child Claude processes) continues running. The user simply restarts ccmux (the client) to reconnect.

### **3.2 Persistence Strategies**

If the *Daemon* crashes, recovery is harder. We must persist state to disk.

#### **3.2.1 State That Must Be Persisted**

1. **Topology:** Window layouts, split sizes, pane titles.  
2. **Metadata:** ClaudeSessionID for each pane, working directories.  
3. **Visual State:** The raw text content of the grids.

#### **3.2.2 Persistence Mechanisms**

* **Write-Ahead Log (WAL):** Log every PTY write operation to disk. On recovery, replay the log into the parser. This guarantees bit-perfect recovery but has high I/O overhead.27  
* **Snapshotting:** Periodically serialize the Grid state. zellij uses KDL for layout persistence.29  
* **Recommendation:** Use a **Hybrid Approach**.  
  * *Topology & Metadata:* Write to a persistent SQLite DB or JSON file on every change (low frequency).  
  * *Visual State:* Do *not* persist full grid state to disk due to performance costs. Instead, rely on ClaudeSessionID. If the Daemon crashes, ccmux restarts, reads the topology, and respawns Claude with \--resume. The *scrollback* is lost, but the *agentic context* is preserved.

### **3.3 Adopting Orphaned PTYs**

A sophisticated recovery strategy involves keeping the PTYs alive even if the Daemon dies.

* **File Descriptor Passing:** On a graceful restart (e.g., update), the Daemon can pass open PTY file descriptors to the new instance via UDS using SCM\_RIGHTS.30  
* **The "Holder" Process:** Similar to shpool 32, a tiny, stable process can hold the PTY master open. The Daemon connects to this holder. If the Daemon crashes, the holder keeps the PTY alive. The new Daemon instance reconnects to the holder. This decouples the volatile application logic (ccmux) from the OS resource (PTY).

**Recommendation:** For v1, focus on Daemon/Client separation. Implementing a shpool-style holder is complex and can be a v2 refinement.

## ---

**4\. Prior Art in Terminal Multiplexers**

Understanding existing tools illuminates the path for ccmux.

### **4.1 tmux**

* **Architecture:** Single C binary acting as server and client. State is in-memory.  
* **Persistence:** Sessions survive client detach. Server crash is fatal.  
* **Lesson:** The separation of server/client is non-negotiable for persistence.34

### **4.2 Zellij**

* **Architecture:** Rust-based. Features a plugin system based on WebAssembly (WASI).36  
* **Strengths:** Modern UI, discoverability, KDL configuration.  
* **Weakness:** The WASM bridge adds complexity and serialization overhead.  
* **Lesson:** Zellij's KDL layout format is excellent. ccmux should adopt a compatible KDL structure for defining layouts.37

### **4.3 WezTerm Mux**

* **Architecture:** wezterm-mux-server handles PTYs.  
* **Strengths:** portable-pty (which ccmux uses) comes from this project. It proves that Rust is viable for high-performance terminal emulation.  
* **Lesson:** WezTerm's font rendering and ligature support are best-in-class but heavy. ccmux should delegate rendering to the terminal emulator it runs *inside* (via Ratatui), avoiding the need to implement font rasterization itself.

### **4.4 shpool**

* **Architecture:** Focuses *only* on persistence, not multiplexing. It effectively implements the "Holder" pattern described in 3.3.  
* **Lesson:** shpool demonstrates that session persistence is a distinct problem from multiplexing. ccmux could potentially use shpool as a dependency to handle the "keep alive" aspect, rather than reinventing it.32

### **Comparison Matrix**

| Feature | tmux | Zellij | ccmux (Target) |
| :---- | :---- | :---- | :---- |
| **Language** | C | Rust | Rust |
| **Parsing** | Custom | alacritty\_terminal (via plugins) | alacritty\_terminal |
| **Semantic Awareness** | None | Limited (via plugins) | **Deep (Claude Integration)** |
| **Config** | Custom DSL | KDL | KDL / TOML |
| **Extensibility** | Scripts | WASM | MCP \+ Rust |

## ---

**5\. Hot-Reload Configuration Patterns**

Users expect to modify keybindings (ccmux.toml) or layouts without restarting the server.

### **5.1 File Watching**

The notify crate is the standard for filesystem monitoring in Rust.

* **Debouncing:** Editors often write atomically (create new file, rename over old). This generates a flurry of events (Create, Rename, Delete). Raw notify usage often triggers multiple reloads. The notify-debouncer-full crate aggregates these events into a stable signal.39

### **5.2 Atomic State Swapping**

Configuration is read often (every render frame for colors/keys) and written rarely.

* **ArcSwap:** This crate provides a lock-free mechanism to swap the global configuration struct. Readers (the render loop) get a snapshot of the config via Arc without blocking writers. This is crucial for maintaining 60fps; a RwLock could cause frame spikes during a reload.41

Rust

use arc\_swap::ArcSwap;  
use std::sync::Arc;

static CONFIG: ArcSwap\<Config\> \= ArcSwap::from\_pointee(Config::default());

// In file watcher thread:  
let new\_config \= load\_config("ccmux.toml")?;  
CONFIG.store(Arc::new(new\_config)); // Atomic swap

// In render loop:  
let cfg \= CONFIG.load(); // Wait-free read  
let color \= cfg.theme.background;

### **5.3 State Migration**

A config reload might change the layout (e.g., removing a pane definition).

* **Migration:** ccmux requires a reconciliation logic. If the new layout has fewer panes, how are existing processes handled?  
  * *Strategy:* "Adopt or Kill". Orphaned processes can be moved to a "detached" list (accessible via a pane picker) rather than being killed immediately. This preserves work if the user makes a configuration error.

## ---

**6\. Claude Code Skills for Structured Output**

To move beyond regex scraping of "Thinking...", ccmux should establish a formal communication protocol with Claude.

### **6.1 Teaching Structured Output via MCP**

The **Model Context Protocol (MCP)** is the robust solution for LLM-tool interaction. Instead of trying to prompt-engineer Claude to output XML tags (which is flaky), ccmux should present itself as an MCP Server.43

#### **6.1.1 ccmux as an MCP Server**

Using mcp-rust-sdk (rmcp), ccmux can expose tools to Claude:

* create\_pane(command: str)  
* read\_pane\_output(pane\_id: int)  
* list\_panes()

**Workflow:**

1. ccmux starts an internal MCP server (stdio or websocket).  
2. When spawning Claude, it passes the MCP configuration.  
3. Claude "sees" these tools. If the user asks "Run the tests in a new split", Claude calls create\_pane("cargo test").  
4. ccmux receives this structured RPC call and executes the layout change deterministically.

### **6.2 Fallback Protocol: The "Sideband"**

For environments where MCP setup is too complex, a text-based sideband protocol can be defined in CLAUDE.md.

* **Format:** XML-like tags that are unlikely to appear in code.  
  XML  
  \<ccmux-action type\="spawn" layout\="vertical"\>  
    cargo test  
  \</ccmux-action\>

* **Parser:** The ccmux stream parser detects these tags, buffers the content, *hides* it from the terminal display (so the user doesn't see raw XML), and executes the action.

### **6.3 Reliability**

Research indicates that "forcing" structured output via JSON mode or schemas is highly reliable.45 However, stream-json is preferred for long-running outputs so that the user sees the text generation incrementally, while the control signals remain structured.

## ---

**7\. Recursion and Orchestration Patterns**

If Claude can spawn new terminals, it can theoretically spawn *itself*. Infinite recursion is a risk.

### **7.1 Supervision Trees (The Actor Model)**

Rust's tokio runtime combined with an actor framework like ractor or simply JoinSet provides the primitives for supervision.47

**Architecture:**

* **Root Supervisor:** Manages the Daemon lifecycle.  
* **Session Supervisor:** Manages a workspace. Children are Panes.  
* **Pane Actor:** Manages a single PTY/Claude process.

**Policies:**

* **Depth Limit:** The Session Supervisor must track the "depth" of the pane tree. If a Claude instance (Depth 1\) requests a new pane, that pane is Depth 2\. Enforce a hard limit (e.g., Depth 3\) to prevent runaway recursion.  
* **Crash Strategy:** If a Pane Actor dies unexpectedly:  
  * *Policy:* Restart (with \--resume) up to 3 times.  
  * *Backoff:* Exponential backoff to prevent CPU thrashing.

### **7.2 Resource Management**

* **Token Quotas:** ccmux can track the token usage reported in the stream-json metadata. It can enforce a "Budget" per session, pausing a pane if it exceeds cost thresholds.  
* **Process Limits:** Use a semaphore (e.g., tokio::sync::Semaphore) to limit the total number of concurrent active Claude processes across the multiplexer.

## ---

**Deliverables**

### **Phase 1: Foundation (The "Blind" Multiplexer)**

* **Core:** Daemon/Client architecture using tokio \+ portable-pty.  
* **UI:** Ratatui client rendering alacritty\_terminal state.  
* **Persistence:** Basic UDS reconnection support.

### **Phase 2: "Claude Awareness"**

* **Parser:** State detection (Thinking, Synthesizing) via stream regex.  
* **Integration:** \--output-format stream-json parsing for clean UI.  
* **Control:** Basic CLAUDE.md sideband parsing for spawn commands.

### **Phase 3: Robustness (The Platform)**

* **MCP:** Full MCP server implementation.  
* **Recovery:** Session ID serialization and auto-resume.  
* **Supervision:** Strict actor hierarchy with recursion limits.

## **Code Examples**

### **Alacritty to Ratatui Adapter**

This function bridges the gap between the emulator state and the UI library.

Rust

use alacritty\_terminal::term::Term;  
use ratatui::buffer::Buffer;  
use ratatui::layout::Rect;  
use ratatui::style::{Color, Style};

pub fn render\_grid\_to\_buffer(term: \&Term\<()\>, buf: &mut Buffer, area: Rect) {  
    let grid \= term.grid();  
    // Use the optimized display iterator which handles scrollback offset automatically  
    for (line\_idx, line) in grid.display\_iter().enumerate() {  
        if line\_idx \>= area.height as usize { break; }  
          
        let y \= area.y \+ line\_idx as u16;  
        for (col\_idx, cell) in line.iter().enumerate() {  
            if col\_idx \>= area.width as usize { break; }  
              
            let x \= area.x \+ col\_idx as u16;  
              
            // Map Alacritty Cell to Ratatui Cell  
            let fg \= convert\_color(cell.fg);  
            let bg \= convert\_color(cell.bg);  
            let style \= Style::default().fg(fg).bg(bg);  
              
            // Ratatui's set\_string or set\_cell is efficient  
            buf.get\_mut(x, y)  
              .set\_char(cell.c)  
              .set\_style(style);  
        }  
    }  
}

// Helper to map Alacritty's specialized Color enum to Ratatui's  
fn convert\_color(c: alacritty\_terminal::vte::ansi::Color) \-\> Color {  
    use alacritty\_terminal::vte::ansi::Color as AColor;  
    match c {  
        AColor::Named(n) \=\> match n {  
            alacritty\_terminal::vte::ansi::NamedColor::Black \=\> Color::Black,  
            alacritty\_terminal::vte::ansi::NamedColor::Red \=\> Color::Red,  
            //... strict mapping of all 16 base colors...  
            \_ \=\> Color::Reset,  
        },  
        AColor::Spec(rgb) \=\> Color::Rgb(rgb.r, rgb.g, rgb.b),  
        AColor::Indexed(i) \=\> Color::Indexed(i),  
    }  
}

### **Hot-Reload Config with ArcSwap**

Rust

use arc\_swap::ArcSwap;  
use std::sync::Arc;  
use notify\_debouncer\_full::{new\_debouncer, notify::\*};

// Global config handle  
static CONFIG: ArcSwap\<AppConfig\> \= ArcSwap::from\_pointee(AppConfig::default());

pub fn start\_config\_watcher() {  
    std::thread::spawn(|| {  
        let (tx, rx) \= std::sync::mpsc::channel();  
        let mut debouncer \= new\_debouncer(Duration::from\_millis(500), None, tx).unwrap();  
          
        debouncer.watcher().watch(Path::new("ccmux.toml"), RecursiveMode::NonRecursive).unwrap();

        for result in rx {  
            match result {  
                Ok(\_) \=\> {  
                    // Reload and swap atomically  
                    if let Ok(new\_cfg) \= load\_config() {  
                        CONFIG.store(Arc::new(new\_cfg));  
                        // Trigger a UI redraw via channel  
                    }  
                },  
                Err(e) \=\> log::error\!("Watch error: {:?}", e),  
            }  
        }  
    });  
}

## **Conclusion**

The architecture of **ccmux** represents a shift from "dumb pipes" to "smart containers." By leveraging portable-pty for low-level interaction, alacritty\_terminal for state correctness, and the Actor model for supervision, ccmux can provide a stable environment for agentic coding. The integration of MCP and structured output parsing transforms the terminal from a passive display into a bidirectional control surface, enabling true collaboration between the developer and the AI agent. The proposed Daemon/Client split and shpool-inspired persistence ensure that this intelligence is robust enough for daily driver usage.

#### **Works cited**

1. gpui\_terminal \- Rust \- Docs.rs, accessed January 7, 2026, [https://docs.rs/gpui-terminal/latest/gpui\_terminal/](https://docs.rs/gpui-terminal/latest/gpui_terminal/)  
2. portable\_pty \- Rust \- Docs.rs, accessed January 7, 2026, [https://docs.rs/portable-pty](https://docs.rs/portable-pty)  
3. portable-pty \- Lib.rs, accessed January 7, 2026, [https://lib.rs/crates/portable-pty](https://lib.rs/crates/portable-pty)  
4. Project | Adventures in Rust \- Hackaday.io, accessed January 7, 2026, [https://hackaday.io/project/194698/logs](https://hackaday.io/project/194698/logs)  
5. Creating a simple Rust daemon that listens to a port \- Stack Overflow, accessed January 7, 2026, [https://stackoverflow.com/questions/26354465/creating-a-simple-rust-daemon-that-listens-to-a-port](https://stackoverflow.com/questions/26354465/creating-a-simple-rust-daemon-that-listens-to-a-port)  
6. Access non standard file descriptors of child process \- Stack Overflow, accessed January 7, 2026, [https://stackoverflow.com/questions/59640128/access-non-standard-file-descriptors-of-child-process](https://stackoverflow.com/questions/59640128/access-non-standard-file-descriptors-of-child-process)  
7. Get child process file descriptor \- help \- The Rust Programming Language Forum, accessed January 7, 2026, [https://users.rust-lang.org/t/get-child-process-file-descriptor/27497](https://users.rust-lang.org/t/get-child-process-file-descriptor/27497)  
8. alacritty/alacritty: A cross-platform, OpenGL terminal emulator. \- GitHub, accessed January 7, 2026, [https://github.com/alacritty/alacritty](https://github.com/alacritty/alacritty)  
9. alacritty\_terminal \- Rust \- Docs.rs, accessed January 7, 2026, [https://docs.rs/alacritty\_terminal/latest/alacritty\_terminal/](https://docs.rs/alacritty_terminal/latest/alacritty_terminal/)  
10. Add support for libsixel · Issue \#910 \- GitHub, accessed January 7, 2026, [https://github.com/alacritty/alacritty/issues/910?timeline\_page=1](https://github.com/alacritty/alacritty/issues/910?timeline_page=1)  
11. fresh-editor — Rust utility // Lib.rs, accessed January 7, 2026, [https://lib.rs/crates/fresh-editor](https://lib.rs/crates/fresh-editor)  
12. Debug mode crash in hint update · Issue \#5315 \- GitHub, accessed January 7, 2026, [https://github.com/alacritty/alacritty/issues/5315](https://github.com/alacritty/alacritty/issues/5315)  
13. Command-line interface — list of Rust libraries/crates // Lib.rs, accessed January 7, 2026, [https://lib.rs/command-line-interface](https://lib.rs/command-line-interface)  
14. Terminal in ratatui \- Rust \- Docs.rs, accessed January 7, 2026, [https://docs.rs/ratatui/latest/ratatui/struct.Terminal.html](https://docs.rs/ratatui/latest/ratatui/struct.Terminal.html)  
15. Rendering under the hood \- Ratatui, accessed January 7, 2026, [https://ratatui.rs/concepts/rendering/under-the-hood/](https://ratatui.rs/concepts/rendering/under-the-hood/)  
16. zed/crates/terminal/src/terminal.rs at main · zed-industries/zed \- GitHub, accessed January 7, 2026, [https://github.com/zed-industries/zed/blob/main/crates/terminal/src/terminal.rs](https://github.com/zed-industries/zed/blob/main/crates/terminal/src/terminal.rs)  
17. ansi-to-tui \- crates.io: Rust Package Registry, accessed January 7, 2026, [https://crates.io/crates/ansi-to-tui/range/%5E8.0.0](https://crates.io/crates/ansi-to-tui/range/%5E8.0.0)  
18. ansi\_to\_tui \- Rust \- Docs.rs, accessed January 7, 2026, [https://docs.rs/ansi-to-tui](https://docs.rs/ansi-to-tui)  
19. alacritty/CHANGELOG.md at master \- GitHub, accessed January 7, 2026, [https://github.com/alacritty/alacritty/blob/master/CHANGELOG.md](https://github.com/alacritty/alacritty/blob/master/CHANGELOG.md)  
20. \[FEATURE\] Expose Claude Code Cognitive Telemetry States via API \#10084 \- GitHub, accessed January 7, 2026, [https://github.com/anthropics/claude-code/issues/10084](https://github.com/anthropics/claude-code/issues/10084)  
21. \[BUG\] Terminal cursor position drift / scrollback corruption on Windows (PowerShell, VS Code) \#14208 \- GitHub, accessed January 7, 2026, [https://github.com/anthropics/claude-code/issues/14208](https://github.com/anthropics/claude-code/issues/14208)  
22. \[Bug\] SHOW-STOPPER: Claude Code 2.0.1 CLI Terminal UI Rendering Corrupted \+ Scrolling Instability \+ Completely Unusable \#8618 \- GitHub, accessed January 7, 2026, [https://github.com/anthropics/claude-code/issues/8618](https://github.com/anthropics/claude-code/issues/8618)  
23. Run Claude Code programmatically \- Claude Code Docs, accessed January 7, 2026, [https://code.claude.com/docs/en/headless](https://code.claude.com/docs/en/headless)  
24. Claude Code: Best practices for agentic coding \- Anthropic, accessed January 7, 2026, [https://www.anthropic.com/engineering/claude-code-best-practices](https://www.anthropic.com/engineering/claude-code-best-practices)  
25. CLI reference \- Claude Code Docs, accessed January 7, 2026, [https://code.claude.com/docs/en/cli-reference](https://code.claude.com/docs/en/cli-reference)  
26. Claude Code settings \- Claude Code Docs, accessed January 7, 2026, [https://code.claude.com/docs/en/settings](https://code.claude.com/docs/en/settings)  
27. Design and Reliability of a User Space Write-Ahead Log in Rust \- arXiv, accessed January 7, 2026, [https://arxiv.org/html/2507.13062](https://arxiv.org/html/2507.13062)  
28. Introducing OkayWAL: A write-ahead log for Rust \- BonsaiDb, accessed January 7, 2026, [https://bonsaidb.io/blog/introducing-okaywal/](https://bonsaidb.io/blog/introducing-okaywal/)  
29. Session Resurrection \- Zellij User Guide, accessed January 7, 2026, [https://zellij.dev/documentation/session-resurrection.html](https://zellij.dev/documentation/session-resurrection.html)  
30. SCM\_RIGHTS \- froghat.ca, accessed January 7, 2026, [https://froghat.ca/2019/05/scm-rights/](https://froghat.ca/2019/05/scm-rights/)  
31. Know your SCM\_RIGHTS \- The Cloudflare Blog, accessed January 7, 2026, [https://blog.cloudflare.com/know-your-scm\_rights/](https://blog.cloudflare.com/know-your-scm_rights/)  
32. shpool \- crates.io: Rust Package Registry, accessed January 7, 2026, [https://crates.io/crates/shpool](https://crates.io/crates/shpool)  
33. Operating systems — list of Rust libraries/crates // Lib.rs, accessed January 7, 2026, [https://lib.rs/os](https://lib.rs/os)  
34. Using tmux to create persistent server sessions \- Brainhack Princeton, accessed January 7, 2026, [https://brainhack-princeton.github.io/handbook/content\_pages/hack\_pages/tmux.html](https://brainhack-princeton.github.io/handbook/content_pages/hack_pages/tmux.html)  
35. Anyone else using tmux as a bootleg orchestration system? : r/ClaudeCode \- Reddit, accessed January 7, 2026, [https://www.reddit.com/r/ClaudeCode/comments/1osd9y1/anyone\_else\_using\_tmux\_as\_a\_bootleg\_orchestration/](https://www.reddit.com/r/ClaudeCode/comments/1osd9y1/anyone_else_using_tmux_as_a_bootleg_orchestration/)  
36. Plugins \- Zellij User Guide, accessed January 7, 2026, [https://zellij.dev/documentation/plugins.html](https://zellij.dev/documentation/plugins.html)  
37. zellij/zellij-utils/assets/config/default.kdl at main \- GitHub, accessed January 7, 2026, [https://github.com/zellij-org/zellij/blob/main/zellij-utils/assets/config/default.kdl](https://github.com/zellij-org/zellij/blob/main/zellij-utils/assets/config/default.kdl)  
38. shell-pool/shpool: Think tmux, then aim... lower \- GitHub, accessed January 7, 2026, [https://github.com/shell-pool/shpool](https://github.com/shell-pool/shpool)  
39. notify\_debouncer\_full \- Rust \- Docs.rs, accessed January 7, 2026, [https://docs.rs/notify-debouncer-full/latest/notify\_debouncer\_full/](https://docs.rs/notify-debouncer-full/latest/notify_debouncer_full/)  
40. notify-debouncer-full \- crates.io: Rust Package Registry, accessed January 7, 2026, [https://crates.io/crates/notify-debouncer-full](https://crates.io/crates/notify-debouncer-full)  
41. arcshift \- Rust \- Docs.rs, accessed January 7, 2026, [https://docs.rs/arcshift](https://docs.rs/arcshift)  
42. Do most people just restart their Rust web servers once every three months? \- Reddit, accessed January 7, 2026, [https://www.reddit.com/r/rust/comments/xaqwj5/do\_most\_people\_just\_restart\_their\_rust\_web/](https://www.reddit.com/r/rust/comments/xaqwj5/do_most_people_just_restart_their_rust_web/)  
43. mcp\_rust\_sdk \- Rust \- Docs.rs, accessed January 7, 2026, [https://docs.rs/mcp\_rust\_sdk](https://docs.rs/mcp_rust_sdk)  
44. How to Build a stdio MCP Server in Rust \- Shuttle.dev, accessed January 7, 2026, [https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust](https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust)  
45. Structured outputs \- Claude Docs, accessed January 7, 2026, [https://platform.claude.com/docs/en/build-with-claude/structured-outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs)  
46. Structured model outputs | OpenAI API, accessed January 7, 2026, [https://platform.openai.com/docs/guides/structured-outputs](https://platform.openai.com/docs/guides/structured-outputs)  
47. ractor-supervisor \- crates.io: Rust Package Registry, accessed January 7, 2026, [https://crates.io/crates/ractor-supervisor](https://crates.io/crates/ractor-supervisor)  
48. Tokio, Futures, and Beyond: Writing Safer & Faster Async Rust | Leapcell, accessed January 7, 2026, [https://leapcell.io/blog/tokio-futures-async-rust](https://leapcell.io/blog/tokio-futures-async-rust)