#!/usr/bin/env python3
"""Build the frozen compiler-only probe in a new isolated directory."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tomllib


PROFILE = {"opt-level": 3, "debug": False, "strip": False, "lto": False,
           "codegen-units": 16, "panic": "unwind", "incremental": False,
           "overflow-checks": False}


def run(command, **kwargs):
    return subprocess.check_output(command, text=True, **kwargs).strip()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkout", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--prepare-only", action="store_true")
    args = parser.parse_args()
    checkout, output = args.checkout.resolve(), args.output.resolve()
    here = Path(__file__).resolve().parent
    assert not run(["git", "-C", str(checkout), "status", "--porcelain", "--untracked-files=no"]), "tracked source must be clean"
    compiler = checkout / "crates/grunkalunka-compiler"
    assert (compiler / "Cargo.toml").is_file()
    locked = tomllib.loads((here / "Cargo.lock").read_text())["package"]
    repo_locked = tomllib.loads((checkout / "Cargo.lock").read_text())["package"]
    identity = lambda package: tuple(package.get(key) for key in ("name", "version", "source", "checksum"))
    assert {identity(package) for package in locked if package["name"] != "lowering-cost-probe"} <= {identity(package) for package in repo_locked}
    output.mkdir(parents=True, exist_ok=False)
    manifest = ('[package]\nname = "lowering-cost-probe"\nversion = "0.0.0"\n'
                'edition = "2024"\npublish = false\n\n[workspace]\n\n'
                '[[bin]]\nname = "lowering-cost-probe"\npath = "probe.rs"\n\n'
                '[dependencies]\ngrunkalunka-compiler = { path = ' + json.dumps(str(compiler)) + ' }\n\n'
                '[profile.release]\n' + ''.join(f'{key} = {json.dumps(value)}\n' for key, value in PROFILE.items()))
    (output / "Cargo.toml").write_text(manifest)
    shutil.copyfile(here / "Cargo.lock", output / "Cargo.lock")
    shutil.copyfile(here / "probe.rs", output / "probe.rs")
    command = ["cargo", "build", "--manifest-path", str(output / "Cargo.toml"), "--release", "--locked"]
    if args.offline:
        command.append("--offline")
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(output / "target")
    receipt = {"source_commit": run(["git", "-C", str(checkout), "rev-parse", "HEAD"]),
               "source_tree": run(["git", "-C", str(checkout), "rev-parse", "HEAD^{tree}"]),
               "harness_sha256": sha(output / "probe.rs"), "lock_sha256": sha(output / "Cargo.lock"),
               "profile": PROFILE, "command": command,
               "build_environment": {key: env.get(key) for key in ("RUSTUP_TOOLCHAIN", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET")},
               "isolated_target": str(output / "target"), "prepared_only": args.prepare_only}
    if not args.prepare_only:
        subprocess.run(command, env=env, check=True)
        binary = output / "target/release/lowering-cost-probe"
        receipt.update(binary_sha256=sha(binary), rustc=run(["rustc", "--version", "--verbose"]), cargo=run(["cargo", "--version"]))
    (output / "build-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"prepared_only": args.prepare_only, "directory": str(output), "command": command}))


if __name__ == "__main__":
    main()
