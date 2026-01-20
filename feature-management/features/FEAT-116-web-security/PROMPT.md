# FEAT-116: Web Security and Authentication

**Priority**: P2
**Component**: ccmux-web
**Effort**: Medium
**Status**: new
**Depends On**: FEAT-113

## Summary

Add security features to the ccmux web interface: TLS/HTTPS support, token-based authentication, and origin validation. Required before exposing ccmux-web beyond localhost.

## Related Features

- **FEAT-113**: Web Interface Core (prerequisite)

## Motivation

- **Remote Access**: Safely expose ccmux over the network
- **Access Control**: Prevent unauthorized terminal access
- **Data Protection**: Encrypt terminal traffic in transit
- **Production Ready**: Meet minimum security standards for deployment

## Security Model

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Unauthorized access | Token authentication |
| Eavesdropping | TLS encryption |
| Cross-site attacks | Origin validation, CSRF tokens |
| Session hijacking | Secure cookies, token expiry |
| Brute force | Rate limiting |

### Trust Boundaries

```
┌─────────────────────────────────────────────────────┐
│ Untrusted: Internet                                 │
│                                                     │
│   Browser ──── TLS ────► ccmux-web                  │
│              (encrypted)    │                       │
│                             │ Auth required         │
│                             ▼                       │
│   ┌─────────────────────────────────────────────┐   │
│   │ Trusted: Local system                       │   │
│   │                                             │   │
│   │   ccmux-web ──► ccmux-client ──► ccmux-server  │
│   │            (unix socket, local PTY)         │   │
│   └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: TLS Support

Enable HTTPS with user-provided certificates.

```toml
# ~/.ccmux/config.toml
[web.tls]
enabled = true
cert_path = "/path/to/cert.pem"
key_path = "/path/to/key.pem"
```

**Implementation** (`ccmux-web/src/tls.rs`):
```rust
use axum_server::tls_rustls::RustlsConfig;

pub async fn configure_tls(config: &WebConfig) -> Option<RustlsConfig> {
    if !config.tls.enabled {
        return None;
    }

    let tls_config = RustlsConfig::from_pem_file(
        &config.tls.cert_path,
        &config.tls.key_path,
    ).await.ok()?;

    Some(tls_config)
}
```

**Server startup**:
```rust
if let Some(tls_config) = configure_tls(&config).await {
    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await?;
} else {
    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await?;
}
```

### Phase 2: Token Authentication

Simple token-based auth for single-user scenarios.

**Token Generation**:
```bash
# Generate a random token
ccmux-web --generate-token
# Output: token: abc123def456...

# Or specify in config
[web.auth]
enabled = true
token = "abc123def456..."  # Or use CCMUX_WEB_TOKEN env var
```

**Authentication Flow**:
```
1. Browser loads /
2. If no valid session, redirect to /login
3. User enters token
4. Server validates, sets secure cookie
5. Subsequent requests include cookie
6. WebSocket upgrade checks cookie
```

**Implementation** (`ccmux-web/src/auth.rs`):
```rust
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn auth_middleware<B>(
    State(config): State<AppConfig>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Skip auth for login page and static assets
    let path = request.uri().path();
    if path == "/login" || path.starts_with("/static/") {
        return Ok(next.run(request).await);
    }

    // Check for valid session cookie
    let session_valid = request
        .headers()
        .get("cookie")
        .and_then(|c| validate_session_cookie(c, &config.auth.token))
        .unwrap_or(false);

    if session_valid {
        Ok(next.run(request).await)
    } else {
        // Redirect to login
        Err(StatusCode::UNAUTHORIZED)
    }
}
```

**Login Page** (`static/login.html`):
```html
<!DOCTYPE html>
<html>
<head>
    <title>ccmux - Login</title>
    <style>
        body {
            background: #1a1a1a;
            color: #fff;
            font-family: system-ui;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
        }
        .login-form {
            background: #2a2a2a;
            padding: 32px;
            border-radius: 8px;
            width: 300px;
        }
        input {
            width: 100%;
            padding: 12px;
            margin: 8px 0;
            border: 1px solid #444;
            border-radius: 4px;
            background: #333;
            color: #fff;
            box-sizing: border-box;
        }
        button {
            width: 100%;
            padding: 12px;
            background: #0066cc;
            border: none;
            border-radius: 4px;
            color: #fff;
            cursor: pointer;
        }
    </style>
</head>
<body>
    <form class="login-form" method="POST" action="/login">
        <h2>ccmux</h2>
        <input type="password" name="token" placeholder="Access token" autofocus>
        <button type="submit">Connect</button>
    </form>
</body>
</html>
```

### Phase 3: Origin Validation

Prevent cross-site WebSocket connections.

```rust
pub fn validate_origin(origin: Option<&str>, config: &WebConfig) -> bool {
    match origin {
        None => false,  // Reject requests without Origin
        Some(origin) => {
            // Check against allowed origins
            config.security.allowed_origins.iter().any(|allowed| {
                origin == allowed || allowed == "*"
            })
        }
    }
}

// In WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(config): State<AppConfig>,
) -> impl IntoResponse {
    let origin = headers.get("origin").and_then(|h| h.to_str().ok());

    if !validate_origin(origin, &config) {
        return StatusCode::FORBIDDEN.into_response();
    }

    ws.on_upgrade(handle_socket)
}
```

### Phase 4: Rate Limiting

Prevent brute force attacks on login.

```rust
use governor::{Quota, RateLimiter};

// 5 attempts per minute per IP
let limiter = RateLimiter::keyed(Quota::per_minute(nonzero!(5u32)));

async fn login_handler(
    State(limiter): State<Arc<RateLimiter<IpAddr>>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Form(credentials): Form<LoginForm>,
) -> impl IntoResponse {
    // Check rate limit
    if limiter.check_key(&addr.ip()).is_err() {
        return (StatusCode::TOO_MANY_REQUESTS, "Too many attempts").into_response();
    }

    // Validate token...
}
```

## Configuration

**Full security configuration**:
```toml
[web]
enabled = true
host = "0.0.0.0"       # Listen on all interfaces (requires auth)
port = 8443

[web.tls]
enabled = true
cert_path = "/etc/ccmux/cert.pem"
key_path = "/etc/ccmux/key.pem"

[web.auth]
enabled = true
token = ""                    # If empty, use CCMUX_WEB_TOKEN env var
session_timeout = 86400       # 24 hours in seconds

[web.security]
allowed_origins = ["https://example.com"]  # Or ["*"] for any
rate_limit_per_minute = 5     # Login attempts
```

## Security Recommendations

### For Users

1. **Always use TLS** when exposing beyond localhost
2. **Use strong tokens** - `openssl rand -hex 32`
3. **Use reverse proxy** (nginx/caddy) for production
4. **Restrict origins** to known domains
5. **Firewall** - limit access to trusted IPs if possible

### Reverse Proxy Example (Caddy)

```
ccmux.example.com {
    reverse_proxy localhost:8080
    # Caddy handles TLS automatically
}
```

### Reverse Proxy Example (nginx)

```nginx
server {
    listen 443 ssl;
    server_name ccmux.example.com;

    ssl_certificate /etc/letsencrypt/live/ccmux.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/ccmux.example.com/privkey.pem;

    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

## Acceptance Criteria

### TLS
- [ ] TLS enabled via config
- [ ] Cert/key loaded from paths
- [ ] HTTPS works with valid certificate
- [ ] HTTP redirects to HTTPS when TLS enabled
- [ ] Clear error if cert/key missing or invalid

### Authentication
- [ ] Token auth enabled via config
- [ ] Token can be set via env var
- [ ] Login page displayed for unauthenticated users
- [ ] Valid token grants access
- [ ] Invalid token shows error
- [ ] Session persists via secure cookie
- [ ] Session expires after timeout
- [ ] Logout clears session

### Origin Validation
- [ ] WebSocket rejects requests without Origin
- [ ] WebSocket rejects requests from disallowed origins
- [ ] Allowed origins configurable

### Rate Limiting
- [ ] Login attempts rate limited per IP
- [ ] Limit configurable
- [ ] Clear error message when rate limited

## Testing

### Security Tests
- [ ] Access without token → redirected to login
- [ ] Invalid token → rejected
- [ ] Valid token → access granted
- [ ] WebSocket from different origin → rejected
- [ ] 6+ rapid login attempts → rate limited
- [ ] Session cookie → HttpOnly, Secure flags set
- [ ] TLS with invalid cert → clear error

### Manual Testing
- [ ] Full flow: login → use terminal → logout
- [ ] Session survives page refresh
- [ ] Session expires after timeout
- [ ] Multiple browsers can authenticate independently

## Out of Scope

- OAuth/OIDC integration
- Multi-user with different permissions
- Audit logging
- Two-factor authentication
- Client certificates

## Future Enhancements

- OAuth2/OIDC for enterprise SSO
- Multiple user accounts with roles
- Audit log of connections and commands
- IP allowlist/denylist
- Yubikey/WebAuthn support
