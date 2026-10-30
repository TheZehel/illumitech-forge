#!/usr/bin/env python3
"""Read-only rendering validation. Never registers, enables or starts a service."""
import ctypes
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET


def main():
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/illumitech-forge" + (".exe" if os.name == "nt" else "")).resolve()
    with tempfile.TemporaryDirectory(prefix="forge service 日本語 ") as directory:
        root = Path(directory).resolve()
        executable = root / ("forge space 日本語.exe" if os.name == "nt" else "forge space 日本語")
        shutil.copy2(binary, executable)
        releases = root / "releases & Unicode 日本語"
        releases.mkdir()
        state = root / "state % $literal 日本語"
        expected = [str(executable), "--state-dir", str(state), "--service-logging", "daemon", "--releases-dir", str(releases)]
        for platform in ["macos", "linux", "windows"]:
            command = [str(binary), "--json", "--state-dir", str(state), "service", "--executable", str(executable), "--releases-dir", str(releases), "--platform", platform]
            preview = json.loads(subprocess.check_output(command))
            assert preview["status"] == "preview"
            assert not state.exists(), "preview must not create state"
            definition = preview["definition"]
            if platform == "macos":
                parsed = plistlib.loads(definition.encode())
                assert parsed["ProgramArguments"] == expected
                assert parsed["Label"] == preview["id"]
                fixture = root / "fixture.plist"
                fixture.write_text(definition, encoding="utf-8")
                if sys.platform == "darwin":
                    subprocess.run(["/usr/bin/plutil", "-lint", str(fixture)], check=True)
            elif platform == "windows":
                parsed = ET.fromstring(definition)
                ns = {"t": "http://schemas.microsoft.com/windows/2004/02/mit/task"}
                assert parsed.find("t:Actions/t:Exec/t:Command", ns).text == expected[0]
                assert len(parsed.find("t:Triggers", ns)) == 0
                assert parsed.find("t:Principals/t:Principal/t:RunLevel", ns).text == "LeastPrivilege"
                args = parsed.find("t:Actions/t:Exec/t:Arguments", ns).text
                assert parsed.find("t:Settings/t:RestartOnFailure", ns) is None
                if os.name == "nt":
                    parse = ctypes.windll.shell32.CommandLineToArgvW
                    parse.argtypes = [ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_int)]
                    parse.restype = ctypes.POINTER(ctypes.c_wchar_p)
                    count = ctypes.c_int()
                    result = parse('"fixture.exe" ' + args, ctypes.byref(count))
                    try:
                        assert list(result[:count.value])[1:] == ["--state-dir", str(state), "service-host", "--releases-dir", str(releases)]
                    finally:
                        free = ctypes.windll.kernel32.LocalFree
                        free.argtypes = [ctypes.c_void_p]
                        free.restype = ctypes.c_void_p
                        free(result)
            else:
                assert "[Install]" not in definition
                assert "%%" in definition and "$$literal" in definition
                fixture = root / (preview["id"] + ".service")
                fixture.write_text(definition, encoding="utf-8")
                if sys.platform.startswith("linux"):
                    subprocess.run(["systemd-analyze", "verify", str(fixture)], check=True)
        print("service fixtures: XML/plist round-trip, escaping, native syntax and preview passed")


if __name__ == "__main__":
    main()
