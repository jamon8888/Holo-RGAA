# Obscura substrate pin

Pinned browser substrate for Holo-RGAA audits: **default-render `obscura 0.2.2`**
(`obscura-x86_64-linux.tar.gz` from upstream releases; stealth/no-render variants
are out of the install path).

## Verify

```bash
./obscura --version          # want: obscura 0.2.2
```

Enforced in three places (keep the version string in sync):

- `install.sh` — `OBSCURA_VERSION` + `verify_obscura_version` (fails the install on mismatch)
- `.github/workflows/ci.yml` — e2e Prepare step greps the pin
- Rust — `ObscuraBridge::binary_version()` + per-run version log in `start_server`;
  `AnalyzePageResult.obscura_version` carries it on every analyze result

## Rollback (exercised 2026-09-06: 0.2.2 → 0.2.0 → 0.2.2, `--version` checked each step)

The previous pin is kept as `obscura-x86_64-linux-0.2.0.tar.gz`. From the repo root:

```bash
tar -xzf obscura-x86_64-linux-0.2.0.tar.gz
cp obscura-x86_64-linux-0.2.0.tar.gz obscura-x86_64-linux.tar.gz
./obscura --version          # want: obscura 0.2.0
```

Re-apply 0.2.2 by extracting the current `obscura-x86_64-linux.tar.gz` from a
checkout at or after the upgrade commit (or re-downloading
`obscura-x86_64-linux.tar.gz` from upstream `v0.2.2`), then re-check `--version`.
