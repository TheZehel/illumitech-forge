#!/usr/bin/env python3
"""Explicit opt-in only, for disposable CI users. Registers and runs a real service."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time


def main():
    if os.environ.get("FORGE_SERVICE_NATIVE_TEST") != "disposable-user-approved":
        raise SystemExit("pending: requires explicit disposable-user approval; no service touched")
    binary = Path(sys.argv[1]).resolve()
    # Keep this directory on failure for native reconciliation; never delete operational evidence.
    root = Path(tempfile.mkdtemp(prefix="forge-native-日本語 ")).resolve()
    state = root / "state"
    releases = root / "releases"
    releases.mkdir()
    base = [str(binary), "--json", "--state-dir", str(state)]

    def service(action):
        return json.loads(subprocess.check_output(base + ["service", action, "--executable", str(binary), "--releases-dir", str(releases)]))

    def heartbeat(previous=None):
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            value = service("status")
            beat = value.get("daemon", {}).get("heartbeat") or {}
            if not value.get("heartbeat_stale", True) and beat.get("running") and beat.get("pid") != previous:
                return beat["pid"]
            time.sleep(2)
        raise AssertionError("native service did not produce a fresh heartbeat")

    service("plan")
    assert not state.exists()
    service("apply")
    service("apply")
    assert not (state / "state.sqlite").exists(), "apply activated the daemon"
    try:
        service("start")
        pid = heartbeat()
        # Pause must survive a crash/restart; no release manifests exist in this queue.
        subprocess.run(base + ["daemon-control", "pause", "--reason", "native smoke"], check=True)
        if os.name == "nt":
            subprocess.run(["taskkill", "/PID", str(pid), "/F"], check=True)
        else:
            os.kill(pid, signal.SIGKILL)
        heartbeat(pid)
        assert service("status")["daemon"]["control"]["paused"] is True
        service("stop")
        service("remove")
        service("remove")
        assert (state / "state.sqlite").exists(), "uninstall deleted operational state"
        assert service("status")["installed"] is False
    finally:
        # Uses the same preservation rules on failures; never broad native cleanup.
        cleanup = subprocess.run(base + ["service", "remove", "--executable", str(binary), "--releases-dir", str(releases)])
        if cleanup.returncode:
            print("native cleanup failed; retained evidence:", root, file=sys.stderr)
    print("native apply/start/status/crash/restart/pause/stop/remove passed; evidence:", root)


if __name__ == "__main__":
    main()
