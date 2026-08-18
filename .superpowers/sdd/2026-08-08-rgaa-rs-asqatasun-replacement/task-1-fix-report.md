# Task 1 Fix Report

**Date:** 2026-08-08

## Changes Made

### Issue 1: Criterion 1.4 classification fix

**File:** `rgaa-rs/crates/rgaa-core/src/criteria.rs:19`

Changed criterion 1.4 ("Alternative CAPTCHA/image-test") classification from `Classification::IaAssiste` to `Classification::Deterministe` to match the official CSV (`grille-rgaa-106.csv`).

### Issue 2: MSRV declaration

**File:** `rgaa-rs/Cargo.toml:13-14`

Added `[workspace.package]` section with `rust-version = "1.80"`.

## Verification

- `cargo check` from `rgaa-rs/` completed successfully (exit 0)
- No errors, no new warnings (only pre-existing `resolver.feature-unification` warning)
- Both files read back and confirmed correct
