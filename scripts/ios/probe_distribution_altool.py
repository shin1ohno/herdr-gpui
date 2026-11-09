"""Credential-free local preflight probe. Never accepts a real IPA or key path."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

import distribution


def main():
    rows = []
    with tempfile.TemporaryDirectory(prefix="herdr-altool-offline-") as temporary:
        root = Path(temporary)
        (root / "tmp").mkdir()
        ipa = root / "nonexistent.ipa"
        key = root / "nonexistent.p8"
        env = dict(HOME=str(root), CFFIXED_USER_HOME=str(root), TMPDIR=str(root / "tmp"),
                   PATH="/usr/bin:/bin:/usr/sbin:/sbin", LC_ALL="C")
        if os.environ.get("DEVELOPER_DIR"):
            env["DEVELOPER_DIR"] = os.environ["DEVELOPER_DIR"]
        metadata = dict(CFBundleIdentifier=distribution.BUNDLE, CFBundleVersion="1",
                        CFBundleShortVersionString="1.0")
        command = distribution.upload_command(ipa, metadata, key, "FIXTUREKEY",
            "00000000-0000-0000-0000-000000000000", "1234567890")
        variants = {
            "legacy_no_auth": ["xcrun", "altool", "--upload-app", "--type", "ios", "--file", str(ipa)],
            "modern_no_auth": ["xcrun", "altool", "--upload-package", str(ipa)],
            "constructed_missing_key": command,
            # Determine whether a missing input preempts argument validation.
            "unknown_flag_control": command + ["--intentionally-invalid-fixture-option"],
        }
        for name, argv in variants.items():
            assert not ipa.exists() and not key.exists()
            result = subprocess.run(argv, cwd=root, env=env, capture_output=True, text=True, timeout=15)
            rows.append(dict(name=name, argv=argv, returncode=result.returncode,
                             stdout=result.stdout, stderr=result.stderr))
    print(json.dumps(dict(scope="local missing-file/auth-path errors only; not upload or complete CLI acceptance",
                          results=rows), indent=2))
    return 0 if all(row["returncode"] != 0 for row in rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
