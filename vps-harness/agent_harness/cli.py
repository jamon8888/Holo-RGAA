"""Command-line entry point for the shared VPS agent harness."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

from . import bootstrap as bootstrap_mod
from . import health, install as install_mod, selftest, setup as setup_mod
from .runners import SubprocessRunner


def install_source_root() -> Path:
    """Locate the source/install root that contains ``agent_harness`` and ``bin``."""
    override = os.environ.get("AGENT_HARNESS_SOURCE_ROOT")
    if override:
        return Path(override).resolve()
    return Path(__file__).resolve().parent.parent


def _add_json(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--json", action="store_true", help="emit machine-readable JSON")


def build_parser() -> argparse.ArgumentParser:
    """Build the top-level argument parser."""
    parser = argparse.ArgumentParser(
        prog="agent-harness",
        description="Shared user-level VPS agent harness (stdlib Python).",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    doctor = sub.add_parser("doctor", help="read-only health report")
    _add_json(doctor)

    bootstrap = sub.add_parser("bootstrap", help="manage agent MCP config")
    group = bootstrap.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true", help="read-only drift report")
    group.add_argument("--apply", action="store_true", help="merge with backup")
    _add_json(bootstrap)

    setup = sub.add_parser("setup", help="validate a repo index and tools (fail closed)")
    setup.add_argument("--root", required=True, help="repository root to validate")
    _add_json(setup)

    selftest_parser = sub.add_parser("self-test", help="agent-specific live proof")
    selftest_parser.add_argument("--agent", required=True, help="one of the six supported agent ids")
    _add_json(selftest_parser)

    installer = sub.add_parser("install", help="install the versioned package and launcher")
    installer.add_argument("--data-home", default=None, help="override harness data dir (for testing)")
    installer.add_argument("--bin-home", default=None, help="override launcher dir (for testing)")
    _add_json(installer)

    return parser


def _human(payload: dict) -> str:
    """Render a compact human-readable summary."""
    lines: list[str] = []

    def walk(obj: object, indent: int = 0) -> None:
        pad = "  " * indent
        if isinstance(obj, dict):
            for key, value in obj.items():
                if key.startswith("_") or key in {"schema", "harness_version"}:
                    continue
                if isinstance(value, (dict, list)):
                    lines.append(f"{pad}{key}:")
                    walk(value, indent + 1)
                else:
                    lines.append(f"{pad}{key}: {value}")
        elif isinstance(obj, list):
            for item in obj:
                if isinstance(item, (dict, list)):
                    walk(item, indent)
                    lines.append("")
                else:
                    lines.append(f"{pad}- {item}")

    walk(payload)
    return "\n".join(lines).rstrip()


def _dispatch(args: argparse.Namespace, runner) -> tuple[dict, int]:
    if args.command == "doctor":
        report = health.doctor(runner)
        return report, health.doctor_exit_code(report)
    if args.command == "bootstrap":
        mode = "apply" if args.apply else "check"
        report = bootstrap_mod.bootstrap(mode, runner)
        return report, bootstrap_mod.bootstrap_exit_code(report)
    if args.command == "setup":
        report = setup_mod.setup_repo(args.root, runner)
        return report, setup_mod.setup_exit_code(report)
    if args.command == "self-test":
        report = selftest.run_self_test(args.agent, runner)
        return report, selftest.selftest_exit_code(report)
    if args.command == "install":
        report = install_mod.install(
            install_source_root(),
            data_home=Path(args.data_home) if args.data_home else None,
            bin_home=Path(args.bin_home) if args.bin_home else None,
        )
        return report, 0
    raise ValueError(f"unhandled command {args.command!r}")  # pragma: no cover


def main(argv: list[str] | None = None) -> int:
    """Run the CLI and return a process exit code."""
    parser = build_parser()
    args = parser.parse_args(argv)
    runner = SubprocessRunner()
    try:
        payload, code = _dispatch(args, runner)
    except (ValueError, RuntimeError, OSError) as exc:
        error = {"status": "error", "message": str(exc)}
        stream = sys.stdout if args.json else sys.stderr
        print(json.dumps(error, indent=2) if args.json else f"error: {exc}", file=stream)
        return 2
    if args.json:
        print(json.dumps(payload, indent=2, ensure_ascii=False))
    else:
        print(_human(payload))
    return code
