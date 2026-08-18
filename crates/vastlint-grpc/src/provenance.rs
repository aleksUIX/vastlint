//! What produced a verdict.
//!
//! A validation result is only reproducible if the ruleset behind it can be
//! identified later, which is the same reason an entity hierarchy needs an
//! as-of date: both are questions about the past being asked of a model that
//! has since moved.
//!
//! Three identifiers, because one is not enough:
//!
//! - `catalog_version` names a release.
//! - `catalog_digest` pins what that release actually carried.
//! - `engine_version` covers the evaluator, since the same ruleset read by a
//!   different parser can reach a different verdict.

use std::sync::OnceLock;

use sha2::{Digest, Sha256};
use vastlint_core::all_rules;

use crate::proto;

static CATALOG_DIGEST: OnceLock<String> = OnceLock::new();

/// Lower-hex of a digest. `sha2` 0.11's output type no longer implements
/// `LowerHex` (`digest` 0.11 dropped it from `hybrid-array`).
fn hex_lower(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Content hash of the rule catalog this binary carries, as `sha256:<hex>`.
///
/// Computed over the linked catalog at first call rather than over the rule
/// source at build time. That distinction is the whole point: it identifies
/// what this binary will actually enforce, so it still differs if rules were
/// compiled out or if a catalog was edited between tagged releases.
///
/// Covers each rule's ID, default severity, and source. A change to any of
/// those changes the verdict a caller can expect, so all three belong in the
/// hash. Rule *messages* are excluded deliberately: reworded prose does not
/// change what passes or fails, and including it would churn the digest on
/// every copy edit.
pub fn catalog_digest() -> &'static str {
    CATALOG_DIGEST.get_or_init(|| {
        let mut hasher = Sha256::new();

        for rule in all_rules() {
            // Field separators, so that ("ab", "c") cannot hash the same as
            // ("a", "bc").
            hasher.update(rule.id.as_bytes());
            hasher.update([0x1f]);
            hasher.update(rule.default_severity.as_str().as_bytes());
            hasher.update([0x1f]);
            hasher.update(rule.source.as_str().as_bytes());
            hasher.update([0x1e]);
        }

        format!("sha256:{}", hex_lower(hasher.finalize()))
    })
}

/// The provenance stamped onto every response.
pub fn provenance() -> proto::Provenance {
    proto::Provenance {
        catalog_version: vastlint_core::VERSION.to_string(),
        catalog_digest: catalog_digest().to_string(),
        engine_version: vastlint_core::VERSION.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_stable_across_calls() {
        assert_eq!(catalog_digest(), catalog_digest());
    }

    #[test]
    fn digest_is_a_prefixed_sha256() {
        let digest = catalog_digest();
        let hex = digest.strip_prefix("sha256:").expect("sha256: prefix");
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn provenance_carries_the_engine_version() {
        let provenance = provenance();
        assert_eq!(provenance.engine_version, vastlint_core::VERSION);
        assert!(!provenance.catalog_version.is_empty());
    }

    /// The digest must depend on the catalog contents, not just its length.
    /// Recomputed here over a permuted catalog to prove the separators do their
    /// job: without them, reordering or re-splitting fields could collide.
    #[test]
    fn field_separators_prevent_collisions() {
        fn digest_of(pairs: &[(&str, &str)]) -> String {
            let mut hasher = Sha256::new();
            for (a, b) in pairs {
                hasher.update(a.as_bytes());
                hasher.update([0x1f]);
                hasher.update(b.as_bytes());
                hasher.update([0x1e]);
            }
            hex_lower(hasher.finalize())
        }

        assert_ne!(digest_of(&[("ab", "c")]), digest_of(&[("a", "bc")]));
    }
}
