//! `rgaa licence …` (#87). Paths are tempdirs, keys are generated per test,
//! and the process environment is never touched.

use std::sync::Arc;

use chrono::{DateTime, Duration, TimeZone, Utc};
use rgaa_license::{
    GateConfig, LicenseCache, LicenseClaims, LicenseError, LicenseFile, LicenseGate, LicenseIssuer,
    ManualClock, Tier, UsageCounter,
};
use rgaa_tui::licence::{
    activate, config_row, deactivate, licence_path, read_token, render_status, source_from,
    LicenceSource,
};

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

fn gate(issuer: &LicenseIssuer, grace: Option<Duration>) -> LicenseGate {
    LicenseGate::new(
        issuer.verifier(),
        Arc::new(UsageCounter::in_memory()),
        Arc::new(ManualClock::new(now())),
        GateConfig {
            grace,
            enforce_quota: false,
            ..GateConfig::default()
        },
    )
}

fn exp() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2027, 11, 19, 0, 0, 0).unwrap()
}

#[test]
fn activate_writes_a_private_file_with_last_validated_now() {
    let issuer = LicenseIssuer::generate().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = licence_path(&dir.path().join("rgaa"));
    let token = issuer.sign(&claims(Tier::Professional, exp())).unwrap();

    let got = activate(&gate(&issuer, None), &token, &path, now()).unwrap();
    assert_eq!(got.tier, Tier::Professional);
    assert_eq!(
        LicenseCache::load_license(&path).unwrap(),
        Some(LicenseFile {
            token,
            last_validated: now()
        })
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[test]
fn invalid_tokens_are_refused_and_nothing_is_written() {
    let issuer = LicenseIssuer::generate().unwrap();
    let stranger = LicenseIssuer::generate().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("license.toml");
    let g = gate(&issuer, None);

    let cases = [
        (
            stranger.sign(&claims(Tier::Team, exp())).unwrap(),
            "la signature ne correspond à aucune clé",
        ),
        ("pas-un-jeton".to_string(), "n'est pas un jeton de licence"),
        (
            issuer
                .sign(&claims(Tier::Team, now() - Duration::days(1)))
                .unwrap(),
            "la licence a expiré le 2026-11-18",
        ),
        (
            {
                let mut future = claims(Tier::Team, exp());
                future.nbf = Some(now() + Duration::days(1));
                issuer.sign(&future).unwrap()
            },
            "la licence n'est pas encore valide",
        ),
    ];
    for (token, why) in cases {
        let err = activate(&g, &token, &path, now()).unwrap_err().to_string();
        assert!(err.starts_with("Clé refusée : "), "{err}");
        assert!(err.contains(why), "{err}");
        assert!(err.contains("Aucun fichier n'a été écrit"), "{err}");
        assert!(!path.exists());
    }
}

#[test]
fn activate_without_verification_key_explains_the_env_var() {
    let issuer = LicenseIssuer::generate().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("license.toml");
    let keyless = LicenseGate::new(
        rgaa_license::LicenseVerifier::new(vec![]),
        Arc::new(UsageCounter::in_memory()),
        Arc::new(ManualClock::new(now())),
        GateConfig::default(),
    );
    let token = issuer.sign(&claims(Tier::Team, exp())).unwrap();
    let err = activate(&keyless, &token, &path, now())
        .unwrap_err()
        .to_string();
    assert!(err.contains("RGAA_LICENSE_PUBLIC_KEYS"), "{err}");
    assert!(!path.exists());
}

#[test]
fn read_token_trims_files_and_rejects_ambiguity() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("key.txt");
    std::fs::write(&f, "  v4.public.abc\n").unwrap();
    assert_eq!(read_token(None, Some(&f)).unwrap(), "v4.public.abc");
    assert_eq!(read_token(Some("t".into()), None).unwrap(), "t");
    assert!(read_token(Some("t".into()), Some(&f)).is_err());
    assert!(read_token(None, None).is_err());
    assert!(read_token(Some("   ".into()), None).is_err());
}

#[test]
fn deactivate_removes_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("license.toml");
    LicenseCache::store_license(
        &path,
        &LicenseFile {
            token: "t".into(),
            last_validated: now(),
        },
    )
    .unwrap();
    assert!(deactivate(&path, false)
        .unwrap()
        .starts_with("Licence supprimée"));
    assert!(!path.exists());
    let again = deactivate(&path, true).unwrap();
    assert!(again.starts_with("Aucune licence à supprimer"), "{again}");
    assert!(
        again.contains("RGAA_LICENSE_KEY est encore défini"),
        "{again}"
    );
}

#[test]
fn source_prefers_env_then_file() {
    let p = std::path::PathBuf::from("/x/license.toml");
    assert_eq!(
        source_from(Some("tok"), Some(p.clone())),
        LicenceSource::Env
    );
    assert_eq!(
        source_from(Some("  "), Some(p.clone())),
        LicenceSource::File(p)
    );
    assert_eq!(source_from(None, None), LicenceSource::NoConfigDir);
}

/// One installed state per `LicenseStatus` variant, rendered.
fn status_for(case: &str) -> (String, String) {
    let issuer = LicenseIssuer::generate().unwrap();
    let g = gate(&issuer, Some(Duration::days(7)));
    let pro = issuer.sign(&claims(Tier::Professional, exp())).unwrap();
    let install = |token: String, last: DateTime<Utc>| {
        g.install(Some(&LicenseFile {
            token,
            last_validated: last,
        }))
    };
    match case {
        "unlicensed" => {}
        "valid" => install(pro, now() - Duration::hours(1)),
        "grace" => install(pro, now() - Duration::days(3)),
        "grace_exceeded" => install(pro, now() - Duration::days(8)),
        "expired" => install(
            issuer
                .sign(&claims(Tier::Team, now() - Duration::days(2)))
                .unwrap(),
            now(),
        ),
        "invalid" => g.install_error(LicenseError::InvalidSignature),
        _ => unreachable!(),
    }
    let snap = g.snapshot();
    assert_eq!(snap.status.as_str(), case);
    let source = LicenceSource::File("/cfg/rgaa/license.toml".into());
    (
        render_status(&snap, now(), &source, true),
        config_row(&snap),
    )
}

#[test]
fn status_renders_every_state() {
    let (s, row) = status_for("unlicensed");
    assert!(s.contains("Offre :      Free"), "{s}");
    assert!(s.contains("aucune licence (offre Free)"), "{s}");
    assert!(s.contains("rgaa licence activate"), "{s}");
    assert!(
        s.contains("Référence de la période de grâce : jamais"),
        "{s}"
    );
    assert!(s.contains("Quota :      0/10 unités"), "{s}");
    assert!(s.contains("(dans 3 h 12 min)"), "{s}");
    assert!(s.contains("Source :     /cfg/rgaa/license.toml"), "{s}");
    assert_eq!(row, "Licence: Free — aucune licence (offre Free)");

    let (s, row) = status_for("valid");
    assert!(s.contains("Offre :      Professional"), "{s}");
    assert!(s.contains("Statut :     valide"), "{s}");
    assert!(s.contains("Titulaire :  org-test"), "{s}");
    assert!(s.contains("Expiration : 2027-11-19 00:00 UTC"), "{s}");
    assert!(
        s.contains("Référence de la période de grâce : 2026-11-19 19:48 UTC"),
        "{s}"
    );
    assert!(s.contains("0/500 unités"), "{s}");
    assert_eq!(
        row,
        "Licence: Professional — valide (expiration : 2027-11-19 00:00 UTC)"
    );

    let (s, _) = status_for("grace");
    assert!(s.contains("hors ligne (période de grâce)"), "{s}");
    assert!(
        s.contains("depuis 3 jours — mode hors ligne, 4 jours restants."),
        "{s}"
    );

    let (s, _) = status_for("grace_exceeded");
    assert!(s.contains("Offre :      Free"), "{s}");
    assert!(
        s.contains("Délai hors ligne expiré : fonctionnalités réduites à l'offre Free."),
        "{s}"
    );

    let (s, _) = status_for("expired");
    assert!(s.contains("Statut :     expirée"), "{s}");
    assert!(s.contains("Expirée le 2026-11-17 20:48 UTC"), "{s}");

    let (s, row) = status_for("invalid");
    assert!(s.contains("Statut :     invalide"), "{s}");
    assert!(s.contains("Raison : la signature ne correspond"), "{s}");
    assert_eq!(row, "Licence: Free — invalide");
}

#[test]
fn env_source_and_disabled_grace_are_explained() {
    let issuer = LicenseIssuer::generate().unwrap();
    let g = gate(&issuer, None);
    g.install(Some(&LicenseFile {
        token: issuer.sign(&claims(Tier::Team, exp())).unwrap(),
        last_validated: now(),
    }));
    let snap = g.snapshot();
    let s = render_status(&snap, now(), &LicenceSource::Env, false);
    assert!(s.contains("Référence de la période de grâce"), "{s}");
    assert!(s.contains("2026-11-19 20:48 UTC"), "{s}");
    assert!(s.contains("Source :     variable RGAA_LICENSE_KEY"), "{s}");
    let s = render_status(&snap, now(), &LicenceSource::File("/f".into()), false);
    assert!(
        s.contains("grâce désactivée : RGAA_LICENSE_SERVER non défini"),
        "{s}"
    );
}
