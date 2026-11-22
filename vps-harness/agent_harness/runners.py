"""Injectable process runners.

All external process calls go through a Runner so unit tests stay fully
offline and deterministic.  The production runner is a thin wrapper around
``subprocess.run`` that converts failures into a ``(returncode, out, err)``
tuple instead of raising.
"""

from __future__ import annotations

import shutil
import subprocess
from typing import Protocol


class Runner(Protocol):
    """Minimal interface used by the health, setup and self-test modules."""

    def which(self, name: str) -> str | None: ...

    def run(self, argv: list[str], *, timeout: int = 15) -> tuple[int, str, str]: ...


class SubprocessRunner:
    """Real runner backed by ``shutil.which`` and ``subprocess.run``."""

    def which(self, name: str) -> str | None:
        return shutil.which(name)

    def run(self, argv: list[str], *, timeout: int = 15) -> tuple[int, str, str]:
        try:
            proc = subprocess.run(
                argv,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=timeout,
                check=False,
            )
            return proc.returncode, proc.stdout, proc.stderr
        except FileNotFoundError:
            return 127, "", f"command not found: {argv[0]}"
        except subprocess.TimeoutExpired:
            return 124, "", f"timeout after {timeout}s: {' '.join(argv)}"
        except OSError as exc:  # pragma: no cover - defensive
            return 125, "", str(exc)
