# ccmux Remote Access Guide

This guide explains how to connect a local `ccmux` client to a remote `ccmux` daemon securely using SSH tunneling. This setup allows you to control a persistent session on a remote server (e.g., a powerful dev machine or cloud instance) from your local terminal.

## Overview

The recommended method for remote access is **SSH Tunneling**. This approach leverages SSH for encryption and authentication, avoiding the need for complex TLS setup or exposing the daemon to the public internet.

**Architecture:**
```
[Local Machine]                                [Remote Machine]
+--------------+                               +--------------+
| ccmux-client | --- TCP (localhost:9999) ---> | SSH Server   |
+--------------+       |                       +------+-------+
                       |                              |
                 (Encrypted Tunnel)                   | TCP (localhost:9999)
                       |                              v
                       |                       +------+-------+
                       +---------------------- | ccmux-server |
                                               +--------------+
```

## Prerequisites

1.  **Remote Machine**: SSH access and `ccmux` installed.
2.  **Local Machine**: `ccmux` installed.

## Step-by-Step Setup

### 1. Start the Remote Daemon

On your remote machine (let's call it `polecats`), start the `ccmux-server` listening on a local TCP port. We bind to `127.0.0.1` (localhost) to ensure the daemon is **not** exposed to the public network directly.

```bash
# On remote machine
ccmux-server --listen-tcp 127.0.0.1:9999
```

*Note: You can verify it's listening with `netstat -tulpn | grep 9999`.*

### 2. Create the SSH Tunnel

On your local machine (let's call it `mayor`), establish an SSH connection that forwards a local port to the remote port.

```bash
# On local machine
ssh -L 9999:127.0.0.1:9999 user@polecats -N
```

*   `-L 9999:127.0.0.1:9999`: Forwards local port 9999 to remote `127.0.0.1:9999`.
*   `-N`: Do not execute a remote command (just forward ports).
*   `user@polecats`: Your SSH login details.

You can run this in a background terminal or add `&` at the end.

### 3. Connect the Local Client

Now, connect your local `ccmux` client to the forwarded local port.

```bash
# On local machine
ccmux --addr tcp://127.0.0.1:9999
```

You should now see the session selection screen or be attached to a session running on the remote machine.

## Automation & Convenience

### Option A: SSH Config

Add the port forwarding to your `~/.ssh/config` to automate the tunnel creation whenever you SSH into the host.

```ssh
# ~/.ssh/config
Host polecats-tunnel
    HostName polecats.example.com
    User your-username
    LocalForward 9999 127.0.0.1:9999
```

Then create the tunnel simply with:
```bash
ssh -N polecats-tunnel
```

### Option B: Environment Variable

If you primarily work with a specific remote instance, set the `CCMUX_ADDR` environment variable in your local shell profile (`.bashrc` or `.zshrc`).

```bash
export CCMUX_ADDR="tcp://127.0.0.1:9999"
```

Now you can just run `ccmux` locally, and it will connect through the tunnel (assuming the tunnel is active).

### Option C: Wrapper Script

Create a script `ccmux-remote` to handle everything:

```bash
#!/bin/bash
# ccmux-remote - Connect to remote ccmux via SSH

REMOTE_HOST="user@polecats"
REMOTE_PORT="9999"
LOCAL_PORT="9999"

# Check if tunnel is already up
if ! lsof -i :$LOCAL_PORT > /dev/null; then
    echo "Establishing SSH tunnel..."
    ssh -L $LOCAL_PORT:127.0.0.1:$REMOTE_PORT $REMOTE_HOST -N -f
fi

# Connect client
ccmux --addr tcp://127.0.0.1:$LOCAL_PORT "$@"
```

## Security Considerations

*   **Bind to Localhost**: Always start the daemon with `--listen-tcp 127.0.0.1:PORT`. Never bind to `0.0.0.0` unless you are on a trusted private network (VPN) and understand the risks.
*   **SSH Keys**: Use SSH key-based authentication for the tunnel to avoid password prompts and enable easy automation.
*   **Firewalls**: Ensure the remote machine allows incoming SSH connections (usually port 22). No other ports need to be opened externally.

## Troubleshooting

### "Connection refused" (Client side)
*   Is the SSH tunnel running? Check `ps aux | grep ssh`.
*   Is the local port correct? Check `ccmux --addr ...`.

### "Channel X: open failed: connect failed: Connection refused" (SSH output)
*   Is the remote daemon running?
*   Is the remote daemon listening on the correct port?
*   Did the remote daemon bind to `127.0.0.1`? (If it bound to `::1` IPv6 only, `127.0.0.1` forwarding might fail depending on OS). Try binding specifically to the IPv4 loopback or check `netstat` on remote.

### Protocol Mismatch
*   Ensure both client and server are running compatible versions of `ccmux`. The protocol version check happens during the handshake.
