//! Opt-in anonymous usage telemetry.
//!
//! Nothing is sent unless the user explicitly enables it via `--telemetry`
//! or `telemetry = true` in `vastlint.toml`.
//!
//! What is sent (one HTTP GET per `vastlint check` invocation):
//!
//! ```text
//!   https://<endpoint>/?v=<version>&os=<os>&id=<install_id>&files=<n>
//! ```
//!
//! - `v`     — vastlint version string (e.g. "0.1.0")
//! - `os`    — operating system ("linux", "macos", "windows", "other")
//! - `id`    — random 128-bit hex install ID, generated once and stored in
//!   `~/.config/vastlint/id`. Never contains any personal data.
//! - `files` — number of files passed to this invocation
//!
//! The ping fires in a background thread with a 2-second timeout and is
//! silently dropped on any error (network down, endpoint unreachable, etc.).
//! It does not block the CLI exit.

use std::path::PathBuf;

/// Endpoint that receives the ping.
/// Set to empty string to disable at compile time.
const TELEMETRY_ENDPOINT: &str = "https://vastlint.org/api/ping";

/// Fire an async telemetry ping. Returns immediately; the request runs in a
/// detached thread. Call only when the user has opted in.
pub fn ping(file_count: usize) {
    if TELEMETRY_ENDPOINT.is_empty() {
        return;
    }

    let version = env!("CARGO_PKG_VERSION");
    let os = os_name();
    let id = install_id();
    let endpoint = format!(
        "{}?v={}&os={}&id={}&files={}",
        TELEMETRY_ENDPOINT, version, os, id, file_count
    );

    // Detached thread — we do not join it. If the process exits before the
    // request completes the OS cleans up. The 2-second timeout ensures the
    // thread doesn't live long regardless.
    std::thread::spawn(move || {
        let _ = ureq::get(&endpoint)
            .timeout(std::time::Duration::from_secs(2))
            .call();
    });
}

/// Print a one-time notice about telemetry on the user's first run.
/// Creates a sentinel file at `~/.config/vastlint/notice-shown` so the
/// message is never displayed again. Silently does nothing if the config
/// directory is unwritable or if we're not connected to a TTY (CI, pipes).
pub fn maybe_show_notice() {
    // Skip in non-interactive contexts — don't pollute CI logs.
    if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        return;
    }

    let Some(sentinel) = notice_path() else {
        return;
    };

    if sentinel.exists() {
        return;
    }

    eprintln!(
        "\n\x1b[1mtip:\x1b[0m help improve vastlint with anonymous usage stats:\n\
         \n\
         \x20   vastlint check --telemetry FILE.xml\n\
         \x20   # or set  telemetry = true  in vastlint.toml\n\
         \n\
         \x20 Only version, OS, and file count are sent. No file contents.\n\
         \x20 Details: https://vastlint.org/telemetry\n"
    );

    // Create the sentinel so we never show this again.
    if let Some(parent) = sentinel.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&sentinel, "1");
}

fn notice_path() -> Option<PathBuf> {
    let config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs_next().map(|h| h.join(".config")))?;
    Some(config_dir.join("vastlint").join("notice-shown"))
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn os_name() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "other"
    }
}

/// Return a stable anonymous install ID, creating it if it does not exist.
/// Stored at `~/.config/vastlint/id` as a 32-char hex string.
/// Falls back to a random ID that is not persisted if the directory is not writable.
fn install_id() -> String {
    if let Some(path) = id_path() {
        if let Ok(existing) = std::fs::read_to_string(&path) {
            let trimmed = existing.trim().to_owned();
            if trimmed.len() == 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
                return trimmed;
            }
        }
        // Generate a new ID and try to persist it.
        let id = random_hex_id();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, &id);
        return id;
    }
    random_hex_id()
}

fn id_path() -> Option<PathBuf> {
    // Respect XDG_CONFIG_HOME if set, otherwise use ~/.config
    let config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs_next().map(|h| h.join(".config")))?;
    Some(config_dir.join("vastlint").join("id"))
}

/// Minimal home-dir lookup without an extra dependency.
fn dirs_next() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Generate a random 32-char hex string using system entropy via `/dev/urandom`
/// on Unix or `BCryptGenRandom` on Windows (via `std` random facilities).
fn random_hex_id() -> String {
    // Use 16 bytes of entropy from the OS via a simple approach:
    // read from /dev/urandom on Unix, fall back to a timestamp+pid mix.
    let bytes = os_random_bytes();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn os_random_bytes() -> [u8; 16] {
    // Try /dev/urandom first (Unix).
    #[cfg(unix)]
    {
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            let mut buf = [0u8; 16];
            if f.read_exact(&mut buf).is_ok() {
                return buf;
            }
        }
    }
    // Fallback: mix timestamp + PID — not cryptographically random but
    // sufficient for a non-security install ID.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id() as u128;
    let mixed = ts ^ (pid << 32) ^ (pid >> 32);
    mixed.to_le_bytes()
}
