"""Check external UIKit linting without compiling or requiring Xcode."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest


SCRIPT = Path(__file__).with_name("lint.sh")


class LintTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="herdr-ios-lint-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        scripts = self.root / "scripts" / "ios"
        scripts.mkdir(parents=True)
        shutil.copyfile(SCRIPT, scripts / "lint.sh")
        (scripts / "test_fixture.py").write_text(textwrap.dedent('''
            from pathlib import Path
            import unittest

            class Fixture(unittest.TestCase):
                def test_isolated_script(self):
                    self.assertTrue(Path(__file__).with_name("lint.sh").is_file())
        '''))
        (scripts / "lib.sh").write_text(
            "ios_export_deployment_target() { export IPHONEOS_DEPLOYMENT_TARGET=17.0; }\n"
        )
        tools = self.root / "tools"
        tools.mkdir()
        self.log = self.root / "calls.jsonl"
        self.manifest = self.root / "cargo git checkout" / "Cargo.toml"
        cargo = tools / "cargo"
        cargo.write_text(f"#!{sys.executable}\n" + textwrap.dedent('''
            import json, os, sys
            args = sys.argv[1:]
            with open(os.environ["HERDR_IOS_LINT_TEST_LOG"], "a") as log:
                log.write(json.dumps(args) + "\\n")
            if args[0] == "metadata":
                packages = [{"name": "unrelated", "manifest_path": "/unrelated/Cargo.toml"}]
                if not os.environ.get("HERDR_IOS_LINT_TEST_MISSING"):
                    packages.append({
                        "name": "herdr-uikit",
                        "manifest_path": os.environ["HERDR_IOS_LINT_TEST_MANIFEST"],
                    })
                print(json.dumps({"packages": packages}))
        '''))
        cargo.chmod(0o755)
        self.env = {
            **os.environ,
            "PATH": f"{tools}{os.pathsep}{os.environ['PATH']}",
            "HERDR_IOS_LINT_TEST_LOG": str(self.log),
            "HERDR_IOS_LINT_TEST_MANIFEST": str(self.manifest),
        }
        self.env.pop("CARGO_TARGET_DIR", None)

    def run_lint(self):
        return subprocess.run(
            ["bash", str(self.root / "scripts" / "ios" / "lint.sh")],
            env=self.env, text=True, capture_output=True, check=False,
        )

    def calls(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def test_lints_resolved_manifest_and_both_application_targets(self):
        result = self.run_lint()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.calls()
        self.assertIn("--filter-platform", calls[0])
        self.assertIn("aarch64-apple-ios-sim", calls[0])
        for target in ("aarch64-apple-ios", "aarch64-apple-ios-sim"):
            platform = next(call for call in calls if "--manifest-path" in call and target in call)
            self.assertEqual(platform[platform.index("--manifest-path") + 1], str(self.manifest))
            self.assertEqual(platform[platform.index("--target-dir") + 1], str(self.root / "target"))
            self.assertIn("--locked", platform)
            self.assertIn("--all-targets", platform)
            application = next(call for call in calls if "herdr-gpui" in call and target in call)
            self.assertIn("--no-default-features", application)

    def test_respects_shared_target_directory(self):
        self.env["CARGO_TARGET_DIR"] = str(self.root / "shared target")
        result = self.run_lint()
        self.assertEqual(result.returncode, 0, result.stderr)
        for call in self.calls():
            if "--manifest-path" in call:
                self.assertEqual(call[call.index("--target-dir") + 1], self.env["CARGO_TARGET_DIR"])

    def test_missing_resolved_platform_stops_before_linting(self):
        self.env["HERDR_IOS_LINT_TEST_MISSING"] = "1"
        result = self.run_lint()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(any(call[0] == "clippy" for call in self.calls()))


if __name__ == "__main__":
    unittest.main()
