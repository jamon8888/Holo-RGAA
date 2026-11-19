//! Licence gate on `rgaa audit …` (#87). Keys are generated per test; no key
//! material is committed, and no test mutates the process environment.

use std::sync::Arc;

use chrono::{DateTime, Duration, TimeZone, Utc};
use rgaa_cli::commands::verify::VerifyArgs;
use rgaa_cli::commands::{dispatch_licensed, AuditCommand, CommonArgs};
use rgaa_cli::license::{admit, messages, LICENSE_EXIT_CODE};
use rgaa_cli::CliError;
use rgaa_license::{
    Feature, GateConfig, LicenseClaims, LicenseFile, LicenseGate, LicenseIssuer, ManualClock, Tier,
    UsageCounter,
};

const URL: &str = "https://example.test/upgrade";

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 19, 20, 48, 0).unwrap()
}

fn claims(tier: Tier, exp: DateTime<Utc>) -> LicenseClaims {
    LicenseClaims {
        sub: "org-test".into(),
        tier,
        iat: now() - Duration::days(30),
        exp,
        nbf: None,
        quota: None,
    }
}

fn gate(issuer: &LicenseIssuer, grace: Option<Duration>, enforce_quota: bool) -> LicenseGate {
    LicenseGate::new(
        issuer.verifier(),
        Arc::new(UsageCounter::in_memory()),
        Arc::new(ManualClock::new(now())),
        GateConfig {
            grace,
            enforce_quota,
            upgrade_url: Some(URL.into()),
            ..GateConfig::default()
        },
    )
}

fn install(gate: &LicenseGate, token: String, last_validated: DateTime<Utc>) {
    gate.install(Some(&LicenseFile {
        token,
        last_validated,
    }));
}

fn refusal(result: Result<Option<String>, CliError>) -> String {
    match result {
        Err(e @ CliError::License(_)) => {
            assert_eq!(e.exit_code(), LICENSE_EXIT_CODE);
            e.to_string()
        }
        other => panic!("expected a licence refusal, got {other:?}"),
    }
}

#[test]
fn free_tier_is_refused_full_audit_with_upgrade_hint() {
    let issuer = LicenseIssuer::generate().unwrap();
    let g = gate(&issuer, None, false);
    let msg = refusal(admit(&g, Feature::FullAudit, now()));
    assert_eq!(
        msg,
        format!(
            "Fonctionnalité réservée à l'offre Professional (offre actuelle : Free). \
             Mettez à niveau : {URL} — puis `rgaa licence activate <clé>`."
        )
    );
}

#[test]
fn professional_licence_is_admitted_without_warning() {
    let issuer = LicenseIssuer::generate().unwrap();
    let g = gate(&issuer, None, false);
    let token = issuer
        .sign(&claims(Tier::Professional, now() + Duration::days(300)))
        .unwrap();
    install(&g, token, now());
    for feature in [Feature::FullAudit, Feature::Igt, Feature::Remediation] {
        assert_eq!(admit(&g, feature, now()).unwrap(), None);
    }
}

#[test]
fn quota_is_not_enforced_locally() {
    let issuer = LicenseIssuer::generate().unwrap();
    let g = gate(&issuer, None, false);
    let token = issuer
        .sign(&claims(Tier::Professional, now() + Duration::days(300)))
        .unwrap();
    install(&g, token, now());
    for _ in 0..(Tier::Professional.daily_quota() + 5) {
        assert!(admit(&g, Feature::FullAudit, now()).is_ok());
    }
}

#[test]
fn expired_licence_is_refused() {
    let issuer = LicenseIssuer::generate().unwrap();
    let g = gate(&issuer, None, false);
    let exp = now() - Duration::days(1);
    install(&g, issuer.sign(&claims(Tier::Team, exp)).unwrap(), now());
    let msg = refusal(admit(&g, Feature::Remediation, now()));
    assert!(
        msg.starts_with(
            "Licence expirée le 2026-11-18 20:48 UTC : fonctionnalités réduites à l'offre Free."
        ),
        "{msg}"
    );
}

#[test]
fn grace_admits_with_warning_then_expires() {
    let issuer = LicenseIssuer::generate().unwrap();
    let g = gate(&issuer, Some(Duration::days(7)), false);
    let token = issuer
        .sign(&claims(Tier::Professional, now() + Duration::days(300)))
        .unwrap();

    install(&g, token.clone(), now() - Duration::days(3));
    assert_eq!(
        admit(&g, Feature::FullAudit, now()).unwrap().as_deref(),
        Some("Licence non vérifiée en ligne depuis 3 jours — mode hors ligne, 4 jours restants.")
    );

    install(&g, token, now() - Duration::days(8));
    assert_eq!(
        refusal(admit(&g, Feature::FullAudit, now())),
        "Délai hors ligne expiré : fonctionnalités réduites à l'offre Free."
    );
}

#[test]
fn invalid_token_is_refused_and_explained() {
    let issuer = LicenseIssuer::generate().unwrap();
    let other = LicenseIssuer::generate().unwrap();
    let g = gate(&issuer, None, false);
    let token = other
        .sign(&claims(Tier::Team, now() + Duration::days(300)))
        .unwrap();
    install(&g, token, now());
    let msg = refusal(admit(&g, Feature::FullAudit, now()));
    assert!(msg.starts_with("Licence invalide : la signature"), "{msg}");
    assert!(msg.contains("rgaa licence deactivate"), "{msg}");
}

#[test]
fn quota_message_shows_usage_and_wait() {
    // Quota refusals only happen on servers; the message is shared.
    let issuer = LicenseIssuer::generate().unwrap();
    let g = gate(&issuer, None, true);
    for _ in 0..10 {
        g.check(Feature::StaticLint, 1).unwrap();
    }
    let denial = g.check(Feature::StaticLint, 1).unwrap_err();
    assert_eq!(
        messages::denial(&denial, now()),
        "Quota journalier atteint (10/10). Réessayez dans 3 h 12 min."
    );
}

#[tokio::test]
async fn dispatch_refuses_before_running_the_command() {
    let issuer = LicenseIssuer::generate().unwrap();
    let g = gate(&issuer, None, false);
    // The issues file does not exist: an admitted command would fail with an
    // execution/input error instead of the licence refusal.
    let command = AuditCommand::Verify(VerifyArgs {
        common: CommonArgs {
            config: None,
            output: None,
            format: None,
            audit_id: None,
            log_file: None,
        },
        issues: "/nonexistent/issues.json".into(),
    });
    let err = dispatch_licensed(command, &g).await.unwrap_err();
    assert!(matches!(err, CliError::License(_)), "{err:?}");
    assert_eq!(err.exit_code(), 4);
}

/// The real binary, with a fresh empty config dir: no licence, so `analyze`
/// is refused with exit code 4 before any browser starts. Environment is set
/// on the child only.
#[test]
fn binary_exits_with_licence_code_when_unlicensed() {
    let dir = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_rgaa-cli"))
        .args(["audit", "analyze", "--url", "https://example.test"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .env_remove("RGAA_LICENSE_KEY")
        .env_remove("RGAA_LICENSE_PUBLIC_KEYS")
        .env_remove("RGAA_LICENSE_SERVER")
        .env_remove("RGAA_LICENSE_UPGRADE_URL")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(4), "stderr: {stderr}");
    assert!(
        stderr.contains("Fonctionnalité réservée à l'offre Professional (offre actuelle : Free)"),
        "{stderr}"
    );
}
