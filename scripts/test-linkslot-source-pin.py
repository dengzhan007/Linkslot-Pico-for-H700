#!/usr/bin/env python3
"""Guard the single immutable Linkslot source pin used by H700 patchers and CI."""
from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
PIN_FILE = ROOT / "integration/linkslot/upstream-snapshot.sha"
PATCHERS = (
    "scripts/apply-linkslot-render-core.py",
    "scripts/apply-linkslot-h700-frontend-core.py",
    "scripts/apply-linkslot-app-router.py",
)
PIN_LOAD = (
    'SNAPSHOT_PIN_FILE = Path(__file__).resolve().parents[1] '
    '/ "integration/linkslot/upstream-snapshot.sha"'
)
WORKFLOW = ".github/workflows/linkslot-lk-rust-feasibility.yml"
RUNTIME_WORKFLOW = ".github/workflows/h700-linkslot-runtime.yml"


def fail(message: str) -> None:
    raise SystemExit(f"FAIL: {message}")


def main() -> int:
    if not PIN_FILE.is_file():
        fail(f"missing shared snapshot pin: {PIN_FILE.relative_to(ROOT)}")
    pin = PIN_FILE.read_text(encoding="ascii").strip()
    if len(pin) != 40 or any(ch not in "0123456789abcdef" for ch in pin):
        fail("shared snapshot pin must be one lowercase 40-character Git SHA")

    for relative in PATCHERS:
        path = ROOT / relative
        text = path.read_text(encoding="utf-8")
        if text.count(PIN_LOAD) != 1:
            fail(f"{relative} does not load the shared snapshot pin exactly once")
        if 'read_text(encoding="ascii").strip()' not in text:
            fail(f"{relative} does not read the shared pin as ASCII")
        guard_variable = "EXPECTED_SOURCE" if "render-core" in relative else "PINNED_SNAPSHOT"
        if f"!= {guard_variable}" not in text:
            fail(f"{relative} does not compare the checkout against the loaded pin")

    feasibility = (ROOT / WORKFLOW).read_text(encoding="utf-8")
    if 'expected_snapshot="$(tr -d \'\\r\\n\' < integration/linkslot/upstream-snapshot.sha)"' not in feasibility:
        fail("feasibility workflow does not read the shared snapshot pin")
    if 'test "$snapshot" = "$expected_snapshot"' not in feasibility:
        fail("feasibility overlay export does not compare against the shared pin")
    if "linkslot-1.0.49-h700-overlay.patch" not in feasibility:
        fail("feasibility workflow does not name the exported patch for 1.0.49")
    if "linkslot-1.0.45-h700-overlay.patch" in feasibility:
        fail("stale 1.0.45 overlay artifact name remains")

    if "snapshot commit %s." not in feasibility:
        fail("overlay README must derive its base commit from the checked snapshot")
    if pin in feasibility:
        fail("overlay workflow must not duplicate the snapshot SHA literal")

    for relative in (WORKFLOW, RUNTIME_WORKFLOW):
        workflow = (ROOT / relative).read_text(encoding="utf-8")
        if workflow.count("scripts/test-linkslot-source-pin.py") < 3:
            fail(f"{relative} must trigger and run the shared source-pin guard")
        if workflow.count("integration/linkslot/upstream-snapshot.sha") < 2:
            fail(f"{relative} must trigger when the shared pin file changes")

    print(f"PASS: all H700 Linkslot patchers and CI share immutable snapshot {pin}")
    print("PASS: workspace version remains 1.0.49; exported patch identity is consistent")
    return 0


if __name__ == "__main__":
    sys.exit(main())
