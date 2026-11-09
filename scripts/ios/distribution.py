"""Opt-in Rust iPad distribution stages; plan never builds, signs, or uploads."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import shlex
import subprocess
import sys
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[2]
BUNDLE = "com.shin1ohno.herdr-gpui"
TEAM = "AL9567565U"
TARGET = "aarch64-apple-ios"


def toolchain():
    return tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]


def cache_key(sdk, xcode):
    inputs = [toolchain(), sdk, xcode, TARGET, "release", "no-default-features",
              hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest()]
    return "ipad-" + hashlib.sha256("\n".join(inputs).encode()).hexdigest()


def run(args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def required_file(variable):
    value = os.environ.get(variable)
    if not value or not Path(value).is_file():
        raise ValueError(f"{variable} must name an existing secret file")
    return Path(value).resolve()


def distribution_entitlements(profile, method, helper):
    derived = helper.entitlements(profile, BUNDLE, TEAM)
    if derived.get("get-task-allow") is not False:
        raise ValueError("a distribution profile must explicitly disable get-task-allow")
    if profile.get("ProvisionsAllDevices"):
        raise ValueError("enterprise profiles are not supported")
    devices = profile.get("ProvisionedDevices")
    if method == "testflight" and devices is not None:
        raise ValueError("TestFlight needs an App Store distribution profile")
    if method == "adhoc" and (not isinstance(devices, list) or not devices):
        raise ValueError("Ad Hoc needs registered devices in its profile")
    return derived


def profile_helper():
    spec = importlib.util.spec_from_file_location("ios_profile", ROOT / "scripts/ios/profile.py")
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    return helper


def package(args):
    # Files are injected by the CI secret provider. No login keychain is opened.
    certificate = required_file("IOS_P12_FILE")
    profile_path = required_file("IOS_PROFILE_FILE")
    password = os.environ.get("IOS_P12_PASSWORD")
    if password is None:
        raise ValueError("IOS_P12_PASSWORD is required (empty is allowed)")
    if not re.fullmatch(r"[1-9][0-9]{0,3}(\.[0-9]{1,2}){0,2}", args.build_number):
        raise ValueError("build number must satisfy Apple's numeric version format")
    app = args.app.resolve()
    info = plistlib.loads((app / "Info.plist").read_bytes())
    if info.get("CFBundleIdentifier") != BUNDLE or info.get("CFBundleSupportedPlatforms") != ["iPhoneOS"]:
        raise ValueError("expected the existing Rust GPUI iPhoneOS bundle identity")
    if any(app.rglob("*.framework")) or any(app.rglob("*.dylib")):
        raise ValueError("nested code needs an explicit signing policy; refusing this bundle")
    if not (app / info["CFBundleExecutable"]).is_file():
        raise ValueError("bundle executable is missing")
    helper = profile_helper()
    try:
        profile = helper.decode(profile_path)
        entitlements = distribution_entitlements(profile, args.method, helper)
    except helper.ProfileError as error:
        raise ValueError("profile validation failed") from error
    if entitlements["application-identifier"] != f"{TEAM}.{BUNDLE}":
        raise ValueError("App ID prefix differs from the verified existing app")
    if args.output.exists():
        raise ValueError("output already exists; choose a fresh IPA path")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="herdr-ci-sign-") as temporary:
        work = Path(temporary)
        work.chmod(0o700)
        keychain = work / "signing.keychain-db"
        key_password = os.urandom(32).hex()
        staged = work / "Payload" / app.name
        staged.parent.mkdir()
        shutil.copytree(app, staged)
        info["CFBundleVersion"] = args.build_number
        (staged / "Info.plist").write_bytes(plistlib.dumps(info))
        shutil.copyfile(profile_path, staged / "embedded.mobileprovision")
        ent = work / "entitlements.plist"
        ent.write_bytes(plistlib.dumps(entitlements))
        search_list = shlex.split(run(["security", "list-keychains", "-d", "user"],
                                      capture_output=True, text=True).stdout)
        created = False
        try:
            run(["security", "create-keychain", "-p", key_password, str(keychain)], capture_output=True)
            created = True
            run(["security", "unlock-keychain", "-p", key_password, str(keychain)], capture_output=True)
            run(["security", "import", str(certificate), "-k", str(keychain), "-P", password,
                 "-T", "/usr/bin/codesign"], capture_output=True)
            run(["security", "set-key-partition-list", "-S", "apple-tool:,apple:,codesign:",
                 "-s", "-k", key_password, str(keychain)], capture_output=True)
            identities = run(["security", "find-identity", "-v", "-p", "codesigning", str(keychain)],
                             capture_output=True, text=True).stdout
            allowed = set(helper.certificate_hashes(profile))
            identity = next((item for item in helper.parse_identities(identities) if item in allowed), None)
            if identity is None:
                raise ValueError("dedicated keychain has no identity matching the profile")
            run(["codesign", "--force", "--keychain", str(keychain), "--sign", identity,
                 "--entitlements", str(ent), "--timestamp=none", str(staged)], capture_output=True)
            run(["codesign", "--verify", "--strict", str(staged)], capture_output=True)
            run(["ditto", "-c", "-k", "--keepParent", str(staged.parent), str(args.output.resolve())])
        finally:
            if created:
                try:
                    run(["security", "delete-keychain", str(keychain)], capture_output=True)
                finally:
                    run(["security", "list-keychains", "-d", "user", "-s", *search_list],
                        capture_output=True)
    print("IPA packaged; App Store acceptance and device update remain unverified")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="stage", required=True)
    sub.add_parser("plan")
    key = sub.add_parser("cache-key")
    key.add_argument("--sdk", required=True)
    key.add_argument("--xcode", required=True)
    sub.add_parser("build")
    pack = sub.add_parser("package")
    pack.add_argument("--app", type=Path, required=True)
    pack.add_argument("--output", type=Path, required=True)
    pack.add_argument("--build-number", required=True)
    pack.add_argument("--method", choices=["testflight", "adhoc"], required=True)
    upload = sub.add_parser("upload")
    upload.add_argument("--ipa", type=Path, required=True)
    args = parser.parse_args()
    start = time.monotonic()
    try:
        if args.stage == "plan":
            print(json.dumps({"execution": False, "toolchain": toolchain(), "target": TARGET,
                              "bundle": BUNDLE, "team": TEAM, "profile": "release",
                              "features": "--no-default-features", "adoption": "choose A or B"}))
        elif args.stage == "cache-key":
            print(cache_key(args.sdk, args.xcode))
        elif args.stage == "build":
            if not os.environ.get("CARGO_TARGET_DIR"):
                raise ValueError("CARGO_TARGET_DIR must name this job's isolated target directory")
            env = dict(os.environ, HERDR_IOS_TEAM="", HERDR_IOS_BUNDLE_ID=BUNDLE)
            result = subprocess.run(["bash", "scripts/ios/bundle.sh", "--package", "sheprd-gpui",
                                     "--bin", "sheprd", "--target", TARGET, "--release",
                                     "--no-default-features", "--bundle-id", BUNDLE],
                                    cwd=ROOT, env=env, text=True, stdout=subprocess.PIPE, check=False)
            # bundle.sh documents status 3 for an assembled unsigned device bundle.
            if result.returncode != 3:
                raise ValueError(f"expected unsigned bundle status 3, received {result.returncode}")
            app = Path(result.stdout.strip().splitlines()[-1])
            if not (app / "Info.plist").is_file():
                raise ValueError("unsigned bundle output is missing")
            print(app)
        elif args.stage == "package":
            package(args)
        else:
            if not args.ipa.is_file():
                raise ValueError("IPA is missing")
            key_file = required_file("ASC_KEY_FILE")
            key_id = os.environ.get("ASC_KEY_ID", "")
            issuer = os.environ.get("ASC_ISSUER_ID", "")
            if not re.fullmatch(r"[A-Z0-9]+", key_id) or not issuer:
                raise ValueError("ASC_KEY_ID and ASC_ISSUER_ID are required")
            with tempfile.TemporaryDirectory(prefix="herdr-ci-upload-") as temporary:
                directory = Path(temporary)
                directory.chmod(0o700)
                destination = directory / f"AuthKey_{key_id}.p8"
                shutil.copyfile(key_file, destination)
                destination.chmod(0o600)
                env = dict(os.environ, API_PRIVATE_KEYS_DIR=str(directory))
                run(["xcrun", "altool", "--upload-app", "--type", "ios", "--file", str(args.ipa),
                     "--apiKey", key_id, "--apiIssuer", issuer], env=env, capture_output=True)
            print("Upload accepted by tool; processing, group delivery and device update are pending")
    except (ValueError, OSError, subprocess.CalledProcessError, plistlib.InvalidFileException) as error:
        # Subprocess arguments may contain passwords. Never format the exception.
        detail = "external command failed; inspect redacted CI diagnostics" if isinstance(error, subprocess.CalledProcessError) else str(error)
        print(f"error: {detail}", file=sys.stderr)
        return 1
    finally:
        print(json.dumps({"stage": args.stage, "elapsed_seconds": round(time.monotonic() - start, 3)}), file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
