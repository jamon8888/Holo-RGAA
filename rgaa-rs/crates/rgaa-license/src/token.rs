//! PASETO v4.public licence tokens (Ed25519).
//!
//! PASETO was chosen over JWT because a v4.public token carries no `alg`
//! header: there is nothing for an attacker to downgrade to `none` or to an
//! HMAC algorithm keyed with the public key. Verification needs only the
//! public key, so it works offline.
//!
//! Time claims (`exp`, `nbf`) are parsed here but *checked* in
//! [`crate::gate`], against the injected [`crate::Clock`]. pasetors' own
//! claim validation reads the system clock, so it is not used.

use chrono::{DateTime, Utc};
use pasetors::keys::{AsymmetricKeyPair, AsymmetricPublicKey, AsymmetricSecretKey, Generate};
use pasetors::token::UntrustedToken;
use pasetors::version4::{PublicToken, V4};
use pasetors::Public;
use serde::{Deserialize, Serialize};

use crate::error::LicenseError;
use crate::tiers::Tier;

/// Binds every token to this product: a v4.public token signed by the same
/// key for another purpose does not verify here.
pub const IMPLICIT_ASSERTION: &[u8] = b"rgaa-license/v1";

/// Comma- or whitespace-separated PASERK public keys (`k4.public.…`) trusted
/// to sign licences. Added to [`BUILTIN_PUBLIC_KEYS`].
pub const PUBLIC_KEYS_ENV: &str = "RGAA_LICENSE_PUBLIC_KEYS";

/// Production verification keys compiled into the binary.
///
/// Empty until the licence service has a signing key (see #87): until then a
/// key must be supplied through [`PUBLIC_KEYS_ENV`].
pub const BUILTIN_PUBLIC_KEYS: &[&str] = &[];

/// The signed payload of a licence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseClaims {
    /// Licensee identifier (account or organisation id).
    pub sub: String,
    pub tier: Tier,
    pub iat: DateTime<Utc>,
    pub exp: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbf: Option<DateTime<Utc>>,
    /// Overrides the tier's daily quota (e.g. a Team contract with more seats).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota: Option<u32>,
}

impl LicenseClaims {
    pub fn daily_quota(&self) -> u32 {
        self.quota.unwrap_or_else(|| self.tier.daily_quota())
    }
}

/// Verifies licence tokens against a set of trusted public keys. Several keys
/// allow rotation: a token verifies if any of them signed it.
pub struct LicenseVerifier {
    keys: Vec<AsymmetricPublicKey<V4>>,
}

impl LicenseVerifier {
    pub fn new(keys: Vec<AsymmetricPublicKey<V4>>) -> Self {
        Self { keys }
    }

    /// Parses PASERK `k4.public.…` keys separated by commas or whitespace.
    pub fn from_paserk(list: &str) -> Result<Self, LicenseError> {
        let keys = list
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(|s| {
                AsymmetricPublicKey::<V4>::try_from(s)
                    .map_err(|e| LicenseError::Malformed(format!("bad public key: {e}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { keys })
    }

    /// Built-in keys plus [`PUBLIC_KEYS_ENV`].
    pub fn from_env() -> Result<Self, LicenseError> {
        let mut list = BUILTIN_PUBLIC_KEYS.join(",");
        if let Ok(extra) = std::env::var(PUBLIC_KEYS_ENV) {
            list.push(',');
            list.push_str(&extra);
        }
        Self::from_paserk(&list)
    }

    pub fn has_keys(&self) -> bool {
        !self.keys.is_empty()
    }

    /// Checks the signature and parses the claims. Does **not** check `exp`
    /// or `nbf`; see the module docs.
    pub fn verify(&self, token: &str) -> Result<LicenseClaims, LicenseError> {
        if self.keys.is_empty() {
            return Err(LicenseError::NoVerificationKey);
        }
        let untrusted = UntrustedToken::<Public, V4>::try_from(token.trim())
            .map_err(|_| LicenseError::Malformed("not a v4.public PASETO token".into()))?;
        let trusted = self
            .keys
            .iter()
            .find_map(|key| {
                PublicToken::verify(key, &untrusted, None, Some(IMPLICIT_ASSERTION)).ok()
            })
            .ok_or(LicenseError::InvalidSignature)?;
        serde_json::from_str(trusted.payload())
            .map_err(|e| LicenseError::Malformed(format!("bad claims: {e}")))
    }
}

/// Signs licences. Used by the licence service and by tests; a binary that
/// only verifies never holds a secret key.
pub struct LicenseIssuer {
    secret: AsymmetricSecretKey<V4>,
    public: AsymmetricPublicKey<V4>,
}

impl LicenseIssuer {
    /// A fresh random key pair. Tests generate one per test rather than
    /// committing key material.
    pub fn generate() -> Result<Self, LicenseError> {
        let kp = AsymmetricKeyPair::<V4>::generate()
            .map_err(|e| LicenseError::Malformed(format!("key generation failed: {e}")))?;
        Ok(Self {
            secret: kp.secret,
            public: kp.public,
        })
    }

    pub fn public_key(&self) -> AsymmetricPublicKey<V4> {
        self.public.clone()
    }

    /// The public key as PASERK (`k4.public.…`), the format
    /// [`PUBLIC_KEYS_ENV`] expects.
    pub fn public_paserk(&self) -> String {
        use pasetors::paserk::FormatAsPaserk;
        let mut out = String::new();
        // Writing to a String cannot fail.
        let _ = self.public.fmt(&mut out);
        out
    }

    pub fn verifier(&self) -> LicenseVerifier {
        LicenseVerifier::new(vec![self.public_key()])
    }

    pub fn sign(&self, claims: &LicenseClaims) -> Result<String, LicenseError> {
        let payload = serde_json::to_vec(claims)
            .map_err(|e| LicenseError::Malformed(format!("claims serialisation: {e}")))?;
        PublicToken::sign(&self.secret, &payload, None, Some(IMPLICIT_ASSERTION))
            .map_err(|e| LicenseError::Malformed(format!("signing failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn claims(tier: Tier) -> LicenseClaims {
        LicenseClaims {
            sub: "org-1".into(),
            tier,
            iat: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            exp: Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap(),
            nbf: None,
            quota: None,
        }
    }

    #[test]
    fn good_signature_round_trips() {
        let issuer = LicenseIssuer::generate().unwrap();
        let token = issuer.sign(&claims(Tier::Team)).unwrap();
        assert!(token.starts_with("v4.public."));
        assert_eq!(
            issuer.verifier().verify(&token).unwrap(),
            claims(Tier::Team)
        );
    }

    #[test]
    fn wrong_key_is_rejected() {
        let a = LicenseIssuer::generate().unwrap();
        let b = LicenseIssuer::generate().unwrap();
        let token = a.sign(&claims(Tier::Team)).unwrap();
        assert_eq!(
            b.verifier().verify(&token),
            Err(LicenseError::InvalidSignature)
        );
    }

    #[test]
    fn any_of_several_keys_verifies_for_rotation() {
        let old = LicenseIssuer::generate().unwrap();
        let new = LicenseIssuer::generate().unwrap();
        let both = LicenseVerifier::new(vec![old.public_key(), new.public_key()]);
        assert!(both.verify(&old.sign(&claims(Tier::Free)).unwrap()).is_ok());
        assert!(both.verify(&new.sign(&claims(Tier::Free)).unwrap()).is_ok());
    }

    #[test]
    fn tampered_payload_is_rejected() {
        let issuer = LicenseIssuer::generate().unwrap();
        let token = issuer.sign(&claims(Tier::Free)).unwrap();
        // Flip one character inside the base64 body.
        let mut bytes = token.into_bytes();
        let i = "v4.public.".len() + 5;
        bytes[i] = if bytes[i] == b'A' { b'B' } else { b'A' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert_eq!(
            issuer.verifier().verify(&tampered),
            Err(LicenseError::InvalidSignature)
        );
    }

    #[test]
    fn other_purposes_and_versions_are_malformed() {
        let issuer = LicenseIssuer::generate().unwrap();
        for bad in [
            "",
            "garbage",
            "v4.local.AAAA",
            "v2.public.AAAA",
            "eyJhbGciOiJub25lIn0.e30.",
        ] {
            assert!(
                matches!(
                    issuer.verifier().verify(bad),
                    Err(LicenseError::Malformed(_))
                ),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn token_for_another_purpose_does_not_verify() {
        let kp = AsymmetricKeyPair::<V4>::generate().unwrap();
        let payload = serde_json::to_vec(&claims(Tier::Team)).unwrap();
        let foreign = PublicToken::sign(&kp.secret, &payload, None, None).unwrap();
        let verifier = LicenseVerifier::new(vec![kp.public]);
        assert_eq!(
            verifier.verify(&foreign),
            Err(LicenseError::InvalidSignature)
        );
    }

    #[test]
    fn unknown_tier_in_signed_claims_is_malformed() {
        let kp = AsymmetricKeyPair::<V4>::generate().unwrap();
        let payload = br#"{"sub":"x","tier":"enterprise","iat":"2026-01-01T00:00:00Z","exp":"2027-01-01T00:00:00Z"}"#;
        let token = PublicToken::sign(&kp.secret, payload, None, Some(IMPLICIT_ASSERTION)).unwrap();
        let verifier = LicenseVerifier::new(vec![kp.public]);
        assert!(matches!(
            verifier.verify(&token),
            Err(LicenseError::Malformed(_))
        ));
    }

    #[test]
    fn no_keys_is_its_own_error() {
        let issuer = LicenseIssuer::generate().unwrap();
        let token = issuer.sign(&claims(Tier::Team)).unwrap();
        assert_eq!(
            LicenseVerifier::new(vec![]).verify(&token),
            Err(LicenseError::NoVerificationKey)
        );
    }

    #[test]
    fn paserk_round_trip() {
        let issuer = LicenseIssuer::generate().unwrap();
        let list = format!("  {} ,", issuer.public_paserk());
        let verifier = LicenseVerifier::from_paserk(&list).unwrap();
        assert!(verifier
            .verify(&issuer.sign(&claims(Tier::Team)).unwrap())
            .is_ok());
        assert!(LicenseVerifier::from_paserk("k4.public.nope").is_err());
        assert!(!LicenseVerifier::from_paserk(" , ").unwrap().has_keys());
    }
}
