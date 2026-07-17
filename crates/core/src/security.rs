//! Enterprise security primitives — **scaffolded, not production-ready**.
//!
//! The spec (§7) calls for encryption at rest/in transit, audit logging, rate
//! limiting, RBAC, and enterprise SSO (SAML, OAuth2). Implementing these
//! correctly requires real cryptography, identity provider integration, and
//! careful compliance review — none of which belong in a first scaffold.
//!
//! Instead, this module defines the **traits and trait-based extension
//! points** plus clearly-marked no-op default stubs. Wire real implementations
//! behind these traits in a deployment. Nothing here should be trusted to
//! enforce security as-is.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Encryption
// ---------------------------------------------------------------------------

/// At-rest encryption for stored documents.
///
/// **Stub:** the default implementation passes bytes through unchanged.
/// A real implementation should use AES-256-GCM (or similar) with a KMS-backed
/// key.
pub trait Encryptor: Send + Sync {
    fn encrypt(&self, plaintext: &[u8]) -> Vec<u8>;
    fn decrypt(&self, ciphertext: &[u8]) -> Vec<u8>;
}

/// Pass-through "encryptor" — **NOT secure.** Use only for local testing.
pub struct NullEncryptor;
impl Encryptor for NullEncryptor {
    fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        plaintext.to_vec()
    }
    fn decrypt(&self, ciphertext: &[u8]) -> Vec<u8> {
        ciphertext.to_vec()
    }
}

// ---------------------------------------------------------------------------
// Audit logging
// ---------------------------------------------------------------------------

/// A single audit record.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub timestamp_unix: u64,
}

/// Append-only audit log sink.
///
/// **Stub:** the default keeps events in memory. A real implementation should
/// write to tamper-evident storage (e.g. append-only file, WORM bucket, or a
/// dedicated audit service).
pub trait AuditLogger: Send + Sync {
    fn record(&self, event: AuditEvent);
}

/// In-memory audit logger — useful for tests and local development only.
pub struct InMemoryAuditLogger {
    pub events: std::sync::Mutex<Vec<AuditEvent>>,
}

impl InMemoryAuditLogger {
    pub fn new() -> Self {
        InMemoryAuditLogger { events: std::sync::Mutex::new(Vec::new()) }
    }
}

impl Default for InMemoryAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLogger for InMemoryAuditLogger {
    fn record(&self, event: AuditEvent) {
        self.events.lock().unwrap().push(event);
    }
}

// ---------------------------------------------------------------------------
// Rate limiting
// ---------------------------------------------------------------------------

/// Simple per-actor token-bucket rate limiter.
///
/// **Stub:** the default allows everything. A real implementation should use a
/// shared, distributed store (e.g. Redis) for correctness across replicas.
pub trait RateLimiter: Send + Sync {
    /// Returns `Ok(())` if the action is allowed, or an error message otherwise.
    fn check(&self, actor: &str) -> std::result::Result<(), String>;
}

/// No-op limiter — **allows everything.**
pub struct UnlimitedRateLimiter;
impl RateLimiter for UnlimitedRateLimiter {
    fn check(&self, _actor: &str) -> std::result::Result<(), String> {
        Ok(())
    }
}

/// Fixed-window per-actor counter. Thread-safe, in-process only.
pub struct FixedWindowRateLimiter {
    pub window_seconds: u64,
    pub max_requests: u32,
    state: std::sync::Mutex<HashMap<String, (u64, u32)>>,
}

impl FixedWindowRateLimiter {
    pub fn new(window_seconds: u64, max_requests: u32) -> Self {
        FixedWindowRateLimiter {
            window_seconds,
            max_requests,
            state: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

impl RateLimiter for FixedWindowRateLimiter {
    fn check(&self, actor: &str) -> std::result::Result<(), String> {
        let now = Self::now();
        let window_start = now - (now % self.window_seconds);
        let mut state = self.state.lock().unwrap();
        let entry = state.entry(actor.to_string()).or_insert((window_start, 0));
        if entry.0 != window_start {
            *entry = (window_start, 0);
        }
        if entry.1 >= self.max_requests {
            return Err("rate limit exceeded".to_string());
        }
        entry.1 += 1;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// RBAC
// ---------------------------------------------------------------------------

/// Roles recognized by the default RBAC implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Viewer,
    Editor,
    Admin,
}

/// Role-based access control.
///
/// **Stub:** the default maps a fixed set of roles to permissions in memory.
/// Real RBAC should integrate with your identity provider and persist
/// assignments in a database.
pub trait Rbac: Send + Sync {
    fn assign(&self, user: &str, role: Role);
    fn has_role(&self, user: &str, role: Role) -> bool;
}

/// In-memory role assignments.
pub struct InMemoryRbac {
    roles: std::sync::Mutex<HashMap<String, Vec<Role>>>,
}

impl InMemoryRbac {
    pub fn new() -> Self {
        InMemoryRbac { roles: std::sync::Mutex::new(HashMap::new()) }
    }
}

impl Default for InMemoryRbac {
    fn default() -> Self {
        Self::new()
    }
}

impl Rbac for InMemoryRbac {
    fn assign(&self, user: &str, role: Role) {
        self.roles
            .lock()
            .unwrap()
            .entry(user.to_string())
            .or_default()
            .push(role);
    }

    fn has_role(&self, user: &str, role: Role) -> bool {
        self.roles
            .lock()
            .unwrap()
            .get(user)
            .map(|roles| roles.contains(&role))
            .unwrap_or(false)
    }
}

/// Enterprise SSO integration points (SAML / OAuth2).
///
/// **Not implemented.** SSO requires identity-provider-specific flows, secure
/// certificate handling, and session management that are out of scope for the
/// initial scaffold. Implement this trait against your IdP of choice.
pub trait SsoProvider: Send + Sync {
    /// Begin the SSO handshake, returning a URL the user should be redirected to.
    fn begin(&self, relay_state: &str) -> std::result::Result<String, String>;
    /// Complete the SSO handshake given the provider's callback payload.
    fn complete(&self, payload: &str) -> std::result::Result<String, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_encryptor_roundtrips() {
        let e = NullEncryptor;
        let ct = e.encrypt(b"hello");
        assert_eq!(e.decrypt(&ct), b"hello");
    }

    #[test]
    fn rate_limiter_blocks_after_max() {
        let rl = FixedWindowRateLimiter::new(60, 2);
        assert!(rl.check("alice").is_ok());
        assert!(rl.check("alice").is_ok());
        assert!(rl.check("alice").is_err());
    }

    #[test]
    fn rbac_assigns_and_checks() {
        let rbac = InMemoryRbac::new();
        rbac.assign("bob", Role::Editor);
        assert!(rbac.has_role("bob", Role::Editor));
        assert!(!rbac.has_role("bob", Role::Admin));
    }

    #[test]
    fn audit_logger_records() {
        let log = InMemoryAuditLogger::new();
        log.record(AuditEvent {
            actor: "alice".into(),
            action: "reduce".into(),
            resource: "doc1".into(),
            timestamp_unix: 0,
        });
        assert_eq!(log.events.lock().unwrap().len(), 1);
    }
}
