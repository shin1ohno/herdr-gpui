"""Device build environment tests; fake CoreDevice and bundling work on any host."""
import json
import unittest

import test_device


class IncrementalTests(unittest.TestCase):
    setUp = test_device.DeviceScriptTests.setUp
    scripted_tree = test_device.DeviceScriptTests.scripted_tree
    run_device = test_device.DeviceScriptTests.run_device

    def test_default_and_explicit_incremental_reach_the_bundle(self):
        script = self.scripted_tree()
        received = self.root / "environment.json"
        self.env["HERDR_DEVICE_TEST_ENV"] = str(received)
        bundle = script.with_name("bundle.sh")
        bundle.write_text(
            "python3 -c 'import json, os; "
            'open(os.environ["HERDR_DEVICE_TEST_ENV"], "w").write(json.dumps(os.environ.get("CARGO_INCREMENTAL")))\'\n'
            'printf "%s\\n" "$HERDR_DEVICE_TEST_APP"\n'
        )
        # Both the default release profile and --dev preserve caller choices.
        for profile in ((), ("--dev",)):
            for supplied in (None, "0", "1", ""):
                with self.subTest(profile=profile, supplied=supplied):
                    self.env.pop("CARGO_INCREMENTAL", None)
                    if supplied is not None:
                        self.env["CARGO_INCREMENTAL"] = supplied
                    result = self.run_device(
                        "--team", "TEAMID1234", "--bundle-id", test_device.BUNDLE,
                        "--no-launch", "--out", str(self.out), *profile, script=script,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(json.loads(received.read_text()),
                                     "1" if supplied is None else supplied)


if __name__ == "__main__":
    unittest.main()
