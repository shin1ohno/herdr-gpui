"""Exercise simulator entitlements and device isolation without building an app."""

import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import textwrap
import unittest


SCRIPT = Path(__file__).with_name("bundle.sh")


@unittest.skipUnless(sys.platform == "darwin", "bundling uses macOS plutil")
class BundleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="herdr-ios-bundle-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        tools = self.root / "tools"
        tools.mkdir()
        self.executable = self.root / "build with spaces" / "herdr-gpui"
        self.executable.parent.mkdir()
        self.executable.write_bytes(b"isolated test executable")
        self.log = self.root / "calls.jsonl"
        self.env = {
            **os.environ,
            "PATH": f"{tools}{os.pathsep}{os.environ['PATH']}",
            "HERDR_BUNDLE_TEST_LOG": str(self.log),
            "HERDR_BUNDLE_TEST_BINARY": str(self.executable),
        }
        self.tool(
            tools,
            "cargo",
            r'''
            import json, os, pathlib, plistlib, sys
            args = sys.argv[1:]
            record = {"tool": "cargo", "args": args}
            for arg in args:
                if arg.startswith("link-arg=-Wl,-sectcreate,__TEXT,__entitlements,"):
                    path = pathlib.Path(arg.split(",", 4)[4])
                    record["entitlements"] = plistlib.loads(path.read_bytes())
                    record["entitlements_path"] = str(path)
            with open(os.environ["HERDR_BUNDLE_TEST_LOG"], "a") as log:
                log.write(json.dumps(record) + "\n")
            kind = "example" if "--example" in args else "bin"
            name = args[args.index("--" + kind) + 1]
            print(json.dumps({
                "reason": "compiler-artifact",
                "executable": os.environ["HERDR_BUNDLE_TEST_BINARY"],
                "target": {"name": name, "kind": [kind]},
                "package_id": "path+file:///isolated#herdr-gpui@0.1.0",
            }))
            ''',
        )
        self.tool(
            tools,
            "codesign",
            r'''
            import json, os, sys
            with open(os.environ["HERDR_BUNDLE_TEST_LOG"], "a") as log:
                log.write(json.dumps({"tool": "codesign", "args": sys.argv[1:]}) + "\n")
            ''',
        )
        self.tool(
            tools,
            "rustup",
            'print("aarch64-apple-ios\\naarch64-apple-ios-sim")',
        )

    @staticmethod
    def tool(directory, name, body):
        path = directory / name
        path.write_text("#!/usr/bin/env python3\n" + textwrap.dedent(body).lstrip())
        path.chmod(0o755)

    def run_bundle(self, *args):
        return subprocess.run(
            ["bash", str(SCRIPT), *args],
            env=self.env,
            text=True,
            capture_output=True,
            check=False,
        )

    def calls(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def test_simulator_links_private_keychain_entitlements_only_into_the_executable(self):
        result = self.run_bundle(
            "--package", "herdr-gpui", "--bin", "herdr-gpui",
            "--bundle-id", "dev.herdr.bundle-test", "--no-default-features",
            "--features", "test-feature",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        call = self.calls()[0]
        self.assertEqual(call["args"][0], "rustc")
        self.assertIn("--locked", call["args"])
        self.assertIn("--no-default-features", call["args"])
        self.assertEqual(call["args"][call["args"].index("--features") + 1], "test-feature")
        self.assertEqual(call["entitlements"], {
            "application-identifier": "HERDRSIM.dev.herdr.bundle-test",
            "keychain-access-groups": ["HERDRSIM.dev.herdr.bundle-test"],
        })
        self.assertFalse(Path(call["entitlements_path"]).exists())
        self.assertTrue((self.executable.parent / "herdr-gpui.app/Info.plist").is_file())
        signatures = [call for call in self.calls() if call["tool"] == "codesign"]
        self.assertTrue(signatures)
        self.assertTrue(all("--entitlements" not in call["args"] for call in signatures))

    def test_device_never_gets_simulator_entitlements_or_ad_hoc_signing(self):
        result = self.run_bundle("--bin", "herdr-gpui", "--target", "aarch64-apple-ios")
        self.assertEqual(result.returncode, 3, result.stderr)
        calls = self.calls()
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0]["args"][0], "build")
        self.assertNotIn("entitlements", calls[0])
        self.assertIn("unsigned device bundle", result.stderr)

    def test_bundle_declares_local_network_access_and_only_ssh_discovery(self):
        result = self.run_bundle("--package", "herdr-gpui", "--bin", "herdr-gpui")
        self.assertEqual(result.returncode, 0, result.stderr)
        info = plistlib.loads(
            (self.executable.parent / "herdr-gpui.app/Info.plist").read_bytes()
        )
        self.assertTrue(info["NSLocalNetworkUsageDescription"].strip())
        self.assertEqual(info["NSBonjourServices"], ["_ssh._tcp"])

    def test_invalid_target_exits_before_invoking_any_build_tool(self):
        result = self.run_bundle("--bin", "herdr-gpui", "--target", "invalid")
        self.assertEqual(result.returncode, 2)
        self.assertFalse(self.log.exists())


if __name__ == "__main__":
    unittest.main()
