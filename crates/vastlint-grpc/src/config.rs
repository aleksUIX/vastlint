//! Server configuration, read from the environment.
//!
//! Every knob has a default that is safe to run with, and every default is
//! stated here rather than scattered across the code. The limiter can be turned
//! off entirely, which exists so the load harness can measure the same server
//! with and without it: a shedding policy nobody has measured is a claim, not a
//! result.

use std::net::SocketAddr;
use std::time::Duration;

/// Everything the server reads from the environment at startup.
#[derive(Debug, Clone)]
pub struct Config {
    pub addr: SocketAddr,
    /// `None` disables the metrics endpoint entirely.
    pub metrics_addr: Option<SocketAddr>,
    pub limit: LimitConfig,
    pub rate_limit: RateLimitConfig,
    /// Largest request body the server will decode.
    pub max_message_bytes: usize,
    /// Header carrying the caller identity used for rate limiting.
    pub caller_header: String,
    /// Async runtime threads. `None` uses tokio's default of one per core.
    pub worker_threads: Option<usize>,
    /// Threads available to run validation.
    ///
    /// This is the server's real capacity, and it is worth being explicit about
    /// why. Validation is CPU-bound and runs on tokio's blocking pool, whose
    /// default ceiling is 512 threads. That default is sized for blocking I/O,
    /// where threads spend their lives waiting. Here they never wait, so 512 of
    /// them on a machine with a dozen cores does not add throughput: it adds
    /// context switching and turns what should be a queue into a stampede.
    /// Sized to the core count instead, so concurrency past capacity shows up
    /// as a decision the limiter gets to make rather than as latency nobody
    /// chose.
    pub blocking_threads: usize,
}

/// Adaptive concurrency limiter settings.
#[derive(Debug, Clone)]
pub struct LimitConfig {
    /// When false, no concurrency limiting and no shedding happens at all. The
    /// A side of the A/B run.
    pub enabled: bool,
    pub initial: usize,
    pub min: usize,
    pub max: usize,
    /// Latency above which a completed request counts as evidence of overload.
    /// Not a request timeout: the request still succeeds, it just votes to
    /// lower the limit.
    pub target_latency: Duration,
    /// Multiplicative decrease factor applied on an overload signal.
    pub backoff_ratio: f64,
}

/// Per-caller token bucket settings.
///
/// Both fields default to zero, which disables the limiter. A rate limit with a
/// number nobody chose is worse than none: it either never fires, or it fires on
/// a legitimate caller who had no way to know the number.
#[derive(Debug, Clone, Default)]
pub struct RateLimitConfig {
    /// Sustained requests per second allowed per caller. 0 disables.
    pub per_second: u32,
    /// Burst capacity. Defaults to one second of sustained rate.
    pub burst: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:50051".parse().expect("valid default addr"),
            metrics_addr: Some("0.0.0.0:9090".parse().expect("valid default metrics addr")),
            limit: LimitConfig::default(),
            rate_limit: RateLimitConfig::default(),
            // 4 MiB, matching tonic's own default. A VAST tag is kilobytes; a
            // 40 MB wrapper chain is an attack, not a creative.
            max_message_bytes: 4 * 1024 * 1024,
            caller_header: "x-vastlint-caller".to_string(),
            worker_threads: None,
            blocking_threads: default_blocking_threads(),
        }
    }
}

/// One validation thread per core.
///
/// Falls back to 4 when the core count is unavailable, which is a number
/// chosen to be small enough not to thrash a constrained container and large
/// enough that the server is not trivially serial.
fn default_blocking_threads() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4)
}

impl Default for LimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            // Starts optimistic. AIMD finds the real ceiling within a few
            // seconds of load, and starting low would shed traffic the server
            // could have served while it climbed.
            initial: 32,
            // Never shed everything. A server that has driven its own limit to
            // zero cannot recover, because it needs completed requests to
            // produce the evidence that would raise the limit again.
            min: 4,
            max: 1024,
            // Calibrate this per deployment. See `LOAD-TEST.md`.
            //
            // The first value here was 50ms, reasoned from the per-tag
            // benchmarks as "twenty-five heavy tags of headroom". Measurement
            // showed that was inert: under saturation the server's own handling
            // time stays under 1ms at p99, so a 50ms trigger never fired and the
            // limiter shed 0.02% of requests while behaving like no limiter at
            // all. 2ms was chosen from the measured distribution instead, and
            // the difference between a number that engages and one that does not
            // is the entire value of having run the experiment.
            target_latency: Duration::from_millis(2),
            // 0.9 rather than the more common 0.5. Halving is right when a
            // breach means a hard dependency failed; here it means the box is
            // busy, and overreacting turns a latency blip into a throughput
            // collapse.
            backoff_ratio: 0.9,
        }
    }
}

impl Config {
    /// Reads configuration from the environment, falling back to defaults.
    ///
    /// A malformed value is an error rather than a silent fallback. Starting
    /// with a default limit because someone typo'd the override is how a server
    /// ends up unprotected while its operator believes otherwise.
    pub fn from_env() -> Result<Self, ConfigError> {
        let defaults = Self::default();

        let metrics_addr = match std::env::var("VASTLINT_METRICS_ADDR") {
            Err(_) => defaults.metrics_addr,
            // An explicitly empty value disables the endpoint. Useful when
            // something else already occupies the port.
            Ok(raw) if raw.trim().is_empty() => None,
            Ok(raw) => Some(parse("VASTLINT_METRICS_ADDR", &raw)?),
        };

        let per_second = env_parse("VASTLINT_RATE_LIMIT_RPS", defaults.rate_limit.per_second)?;
        let burst = match std::env::var("VASTLINT_RATE_LIMIT_BURST") {
            Ok(raw) => parse("VASTLINT_RATE_LIMIT_BURST", &raw)?,
            // One second of sustained rate. Enough to absorb a caller that
            // batches its requests without letting it sustain the burst rate.
            Err(_) => per_second,
        };

        let config = Self {
            addr: match std::env::var("VASTLINT_GRPC_ADDR") {
                Ok(raw) => parse("VASTLINT_GRPC_ADDR", &raw)?,
                Err(_) => defaults.addr,
            },
            metrics_addr,
            limit: LimitConfig {
                enabled: env_flag("VASTLINT_LIMIT_ENABLED", defaults.limit.enabled)?,
                initial: env_parse("VASTLINT_LIMIT_INITIAL", defaults.limit.initial)?,
                min: env_parse("VASTLINT_LIMIT_MIN", defaults.limit.min)?,
                max: env_parse("VASTLINT_LIMIT_MAX", defaults.limit.max)?,
                target_latency: Duration::from_millis(env_parse(
                    "VASTLINT_LIMIT_TARGET_LATENCY_MS",
                    defaults.limit.target_latency.as_millis() as u64,
                )?),
                backoff_ratio: env_parse("VASTLINT_LIMIT_BACKOFF", defaults.limit.backoff_ratio)?,
            },
            rate_limit: RateLimitConfig { per_second, burst },
            max_message_bytes: env_parse("VASTLINT_MAX_MESSAGE_BYTES", defaults.max_message_bytes)?,
            caller_header: std::env::var("VASTLINT_CALLER_HEADER")
                .unwrap_or(defaults.caller_header),
            worker_threads: match std::env::var("VASTLINT_WORKER_THREADS") {
                Ok(raw) => Some(parse("VASTLINT_WORKER_THREADS", &raw)?),
                Err(_) => defaults.worker_threads,
            },
            blocking_threads: env_parse("VASTLINT_BLOCKING_THREADS", defaults.blocking_threads)?,
        };

        config.validate()?;
        Ok(config)
    }

    /// Rejects combinations that would leave the server in a state it cannot
    /// recover from, rather than discovering them under load.
    fn validate(&self) -> Result<(), ConfigError> {
        if self.limit.min == 0 {
            return Err(ConfigError::Invalid {
                name: "VASTLINT_LIMIT_MIN",
                reason: "must be at least 1: a limit of zero sheds every request, including the \
                         ones whose completion would raise the limit again",
            });
        }

        if self.limit.min > self.limit.max {
            return Err(ConfigError::Invalid {
                name: "VASTLINT_LIMIT_MIN",
                reason: "must not exceed VASTLINT_LIMIT_MAX",
            });
        }

        if !(0.0..1.0).contains(&self.limit.backoff_ratio) {
            return Err(ConfigError::Invalid {
                name: "VASTLINT_LIMIT_BACKOFF",
                reason: "must be in [0.0, 1.0): 1.0 or above never decreases the limit, so \
                         the limiter would only ever grow",
            });
        }

        if self.max_message_bytes == 0 {
            return Err(ConfigError::Invalid {
                name: "VASTLINT_MAX_MESSAGE_BYTES",
                reason: "must be at least 1",
            });
        }

        if self.blocking_threads == 0 {
            return Err(ConfigError::Invalid {
                name: "VASTLINT_BLOCKING_THREADS",
                reason: "must be at least 1, or no validation can run at all",
            });
        }

        if self.worker_threads == Some(0) {
            return Err(ConfigError::Invalid {
                name: "VASTLINT_WORKER_THREADS",
                reason: "must be at least 1",
            });
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Unparseable {
        name: &'static str,
        value: String,
    },
    Invalid {
        name: &'static str,
        reason: &'static str,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unparseable { name, value } => {
                write!(f, "{name} is not a valid value: {value:?}")
            }
            Self::Invalid { name, reason } => write!(f, "{name} {reason}"),
        }
    }
}

impl std::error::Error for ConfigError {}

fn parse<T: std::str::FromStr>(name: &'static str, raw: &str) -> Result<T, ConfigError> {
    raw.trim().parse().map_err(|_| ConfigError::Unparseable {
        name,
        value: raw.to_string(),
    })
}

fn env_parse<T: std::str::FromStr>(name: &'static str, fallback: T) -> Result<T, ConfigError> {
    match std::env::var(name) {
        Ok(raw) => parse(name, &raw),
        Err(_) => Ok(fallback),
    }
}

fn env_flag(name: &'static str, fallback: bool) -> Result<bool, ConfigError> {
    match std::env::var(name) {
        Err(_) => Ok(fallback),
        Ok(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(ConfigError::Unparseable { name, value: raw }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        Config::default().validate().expect("defaults validate");
    }

    /// The limiter needs completed requests to raise its own limit, so a floor
    /// of zero is an absorbing state: once there, no request is admitted, so no
    /// evidence arrives, so the limit never rises.
    #[test]
    fn a_minimum_limit_of_zero_is_rejected() {
        let mut config = Config::default();
        config.limit.min = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn an_inverted_limit_range_is_rejected() {
        let mut config = Config::default();
        config.limit.min = 100;
        config.limit.max = 10;
        assert!(config.validate().is_err());
    }

    #[test]
    fn a_backoff_ratio_that_never_decreases_is_rejected() {
        let mut config = Config::default();
        config.limit.backoff_ratio = 1.0;
        assert!(config.validate().is_err());

        config.limit.backoff_ratio = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn a_backoff_ratio_of_zero_is_allowed_if_aggressive() {
        let mut config = Config::default();
        config.limit.backoff_ratio = 0.0;
        // Collapses straight to the floor on any overload signal. Extreme, but
        // it is a choice an operator is entitled to make.
        assert!(config.validate().is_ok());
    }

    #[test]
    fn a_server_with_no_validation_threads_is_rejected() {
        let config = Config {
            blocking_threads: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    /// The blocking pool is the server's real capacity, so it must not silently
    /// inherit tokio's I/O-shaped default of 512 threads for CPU-bound work.
    #[test]
    fn the_default_validation_pool_is_sized_to_the_machine() {
        let config = Config::default();
        assert!(config.blocking_threads >= 1);
        assert!(
            config.blocking_threads <= 256,
            "a CPU-bound pool should track cores, not tokio's blocking default"
        );
    }

    #[test]
    fn flags_accept_the_usual_spellings() {
        assert!(env_flag("VASTLINT_TEST_MISSING_FLAG", true).unwrap());
        assert!(!env_flag("VASTLINT_TEST_MISSING_FLAG", false).unwrap());
    }
}
