//! HMAC-signed webhook delivery for batch completion.
//!
//! A webhook tells a third party that their batch finished. Without a
//! signature, anyone who learns the callback URL can post a forged
//! "your batch passed" — and the receiver, which by construction accepts
//! unauthenticated inbound POSTs, has no way to tell. The signature is what
//! makes the callback evidence rather than a rumour.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Header carrying the signature, as `t=<unix_seconds>,v1=<hex>`.
pub const SIGNATURE_HEADER: &str = "x-rgaa-signature";

/// How far a timestamp may be from the receiver's clock before the delivery
/// is refused as a replay.
///
/// A signature alone does not stop replay: a captured "batch passed" body
/// stays valid forever, because it is still correctly signed. Binding the
/// timestamp into the signed material and bounding its age is what closes
/// that. Five minutes is the usual allowance for clock skew between two
/// machines that are not running NTP against each other.
pub const MAX_TIMESTAMP_SKEW_SECS: u64 = 300;

/// Why a delivery was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignatureError {
    #[error("signature header missing")]
    Missing,
    #[error("signature header malformed: expected 't=<unix_seconds>,v1=<hex>'")]
    Malformed,
    #[error("signature timestamp is {age_secs}s from now, more than the {max_secs}s allowed")]
    StaleTimestamp { age_secs: u64, max_secs: u64 },
    #[error("signature does not match the body")]
    Mismatch,
}

/// The signed material: the timestamp and the body, joined by a `.`.
///
/// The separator matters. Signing `timestamp || body` with no delimiter
/// lets `("12", "3{...}")` and `("123", "{...}")` produce identical input,
/// so a signature made for one timestamp is valid for another. A byte that
/// cannot appear in a decimal timestamp removes the ambiguity.
fn signing_input(timestamp: u64, body: &[u8]) -> Vec<u8> {
    let mut input = timestamp.to_string().into_bytes();
    input.push(b'.');
    input.extend_from_slice(body);
    input
}

/// Signs `body` and returns the full header value.
///
/// `body` must be the exact bytes put on the wire. Signing a value and then
/// re-serializing it to send produces a signature over bytes the receiver
/// never sees: map ordering, float formatting and escaping all differ
/// between serializers, and the receiver — who can only hash what arrived —
/// would reject every delivery.
pub fn sign(secret: &[u8], timestamp: u64, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC-SHA256 accepts a key of any length");
    mac.update(&signing_input(timestamp, body));
    let digest = mac.finalize().into_bytes();
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("t={timestamp},v1={hex}")
}

/// Verifies a received `x-rgaa-signature` against the raw body.
///
/// `now` is passed in rather than read from the clock so the staleness
/// rule can be tested without sleeping.
pub fn verify(
    secret: &[u8],
    header: Option<&str>,
    body: &[u8],
    now: u64,
) -> Result<(), SignatureError> {
    let header = header.ok_or(SignatureError::Missing)?;

    let mut timestamp: Option<u64> = None;
    let mut provided: Option<&str> = None;
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", value)) => timestamp = value.parse().ok(),
            Some(("v1", value)) => provided = Some(value),
            _ => {}
        }
    }
    let (timestamp, provided) = match (timestamp, provided) {
        (Some(t), Some(v)) if !v.is_empty() => (t, v),
        _ => return Err(SignatureError::Malformed),
    };

    // Absolute difference: a timestamp far in the *future* is as suspect as
    // a stale one, and `now - timestamp` alone would underflow on it.
    let age = now.abs_diff(timestamp);
    if age > MAX_TIMESTAMP_SKEW_SECS {
        return Err(SignatureError::StaleTimestamp {
            age_secs: age,
            max_secs: MAX_TIMESTAMP_SKEW_SECS,
        });
    }

    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC-SHA256 accepts a key of any length");
    mac.update(&signing_input(timestamp, body));
    let expected = mac.finalize().into_bytes();

    let Some(provided) = decode_hex(provided) else {
        return Err(SignatureError::Malformed);
    };
    // Length check first: `ct_eq` on different lengths is not meaningful,
    // and comparing with `==` here would leak the digest one byte at a time
    // through response timing.
    if provided.len() != expected.len() {
        return Err(SignatureError::Mismatch);
    }
    use subtle::ConstantTimeEq;
    if provided.ct_eq(&expected).into() {
        Ok(())
    } else {
        Err(SignatureError::Mismatch)
    }
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"a-shared-secret";
    const BODY: &[u8] = br#"{"batch_id":"b-1","status":"completed"}"#;
    const NOW: u64 = 1_700_000_000;

    #[test]
    fn a_signature_this_service_produced_verifies() {
        let header = sign(SECRET, NOW, BODY);
        assert_eq!(verify(SECRET, Some(&header), BODY, NOW), Ok(()));
    }

    #[test]
    fn a_body_altered_in_flight_is_refused() {
        let header = sign(SECRET, NOW, BODY);
        let tampered = br#"{"batch_id":"b-1","status":"passed"}"#;
        assert_eq!(
            verify(SECRET, Some(&header), tampered, NOW),
            Err(SignatureError::Mismatch)
        );
    }

    #[test]
    fn a_signature_from_a_different_secret_is_refused() {
        let header = sign(b"someone-elses-secret", NOW, BODY);
        assert_eq!(
            verify(SECRET, Some(&header), BODY, NOW),
            Err(SignatureError::Mismatch)
        );
    }

    /// A correctly signed body stays correctly signed forever, so without a
    /// bound on the timestamp a captured delivery can be replayed at will.
    #[test]
    fn a_captured_delivery_cannot_be_replayed_later() {
        let header = sign(SECRET, NOW, BODY);
        let much_later = NOW + MAX_TIMESTAMP_SKEW_SECS + 1;
        assert!(matches!(
            verify(SECRET, Some(&header), BODY, much_later),
            Err(SignatureError::StaleTimestamp { .. })
        ));
    }

    /// A timestamp far in the future is as suspect as a stale one, and is
    /// also the case where a naive `now - timestamp` underflows.
    #[test]
    fn a_timestamp_from_the_future_is_refused_not_wrapped() {
        let header = sign(SECRET, NOW + 10_000, BODY);
        assert!(matches!(
            verify(SECRET, Some(&header), BODY, NOW),
            Err(SignatureError::StaleTimestamp { .. })
        ));
    }

    /// Small clock differences between two machines are normal and must not
    /// break delivery.
    #[test]
    fn ordinary_clock_skew_is_tolerated() {
        let header = sign(SECRET, NOW, BODY);
        assert_eq!(verify(SECRET, Some(&header), BODY, NOW + 30), Ok(()));
        assert_eq!(verify(SECRET, Some(&header), BODY, NOW - 30), Ok(()));
    }

    /// The `.` delimiter exists so a signature made for one timestamp is not
    /// valid for another. Without it, `t=12` over `3{body}` and `t=123` over
    /// `{body}` hash identical bytes.
    #[test]
    fn the_timestamp_cannot_be_shifted_into_the_body() {
        let shifted = sign(SECRET, 12, b"3.{}");
        let signature = shifted.split("v1=").nth(1).expect("v1");
        let forged = format!("t=123,v1={signature}");
        assert_eq!(
            verify(SECRET, Some(&forged), b"{}", 123),
            Err(SignatureError::Mismatch)
        );
    }

    /// Pins the exact bytes a receiver must reproduce.
    ///
    /// `docs/batch-webhooks.md` publishes Node and Python verifiers. If the
    /// signing input here ever changes — a different separator, the
    /// timestamp dropped, the body hashed after re-serialization — those
    /// published snippets silently stop working for everyone who copied
    /// them, and the only symptom is every delivery being rejected as
    /// forged. This vector was produced independently with `hmac`/`hashlib`
    /// in Python and `node:crypto` in Node; all three agree.
    #[test]
    fn the_wire_format_matches_the_published_verifiers() {
        let header = sign(SECRET, NOW, BODY);
        assert_eq!(
            header,
            "t=1700000000,\
             v1=78a88c99e8aebbef7624f16ea0fee62a9eac1f657665ca08ecd769b1488815eb"
        );
    }

    #[test]
    fn a_missing_or_malformed_header_is_named_as_such() {
        assert_eq!(
            verify(SECRET, None, BODY, NOW),
            Err(SignatureError::Missing)
        );
        assert_eq!(
            verify(SECRET, Some("garbage"), BODY, NOW),
            Err(SignatureError::Malformed)
        );
        assert_eq!(
            verify(SECRET, Some("t=1700000000"), BODY, NOW),
            Err(SignatureError::Malformed)
        );
        assert_eq!(
            verify(SECRET, Some("t=1700000000,v1="), BODY, NOW),
            Err(SignatureError::Malformed)
        );
        assert_eq!(
            verify(SECRET, Some("t=1700000000,v1=zz"), BODY, NOW),
            Err(SignatureError::Malformed)
        );
    }

    /// An odd-length or truncated digest must be refused, not silently
    /// compared against a prefix.
    #[test]
    fn a_truncated_signature_is_refused() {
        let header = sign(SECRET, NOW, BODY);
        let digest = header.split("v1=").nth(1).expect("v1");
        let truncated = format!("t={NOW},v1={}", &digest[..digest.len() - 2]);
        assert_eq!(
            verify(SECRET, Some(&truncated), BODY, NOW),
            Err(SignatureError::Mismatch)
        );
    }
}
