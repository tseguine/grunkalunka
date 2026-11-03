#!/usr/bin/env python3
"""Build the unchanged parity harness in an isolated, clean repository checkout."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess


def command(arguments, cwd):
    return subprocess.check_output(arguments, cwd=cwd, text=True, encoding="utf-8").strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, required=True,
                        help="an exclusive Cargo target directory for this source checkout")
    parser.add_argument("--output", type=Path, required=True,
                        help="fresh directory for the immutable binary and build receipt")
    parser.add_argument("--expect-commit", required=True)
    parser.add_argument("--cargo", default="cargo")
    args = parser.parse_args()
    repo, target, output = args.repo.resolve(), args.target_dir.resolve(), args.output.resolve()
    commit = command(["git", "rev-parse", "HEAD"], repo)
    if commit != args.expect_commit:
        raise ValueError(f"wrong source commit: {commit}")
    if command(["git", "status", "--porcelain", "--untracked-files=no"], repo):
        raise ValueError("tracked source changes in build checkout")
    if target.exists():
        raise ValueError("target directory must be new to prevent stale cross-worktree artifacts")
    source = Path(__file__).with_name("probe.rs")
    example = repo / "crates/grunkalunka-compiler/examples/lowering_parity_snapshot.rs"
    example.parent.mkdir(exist_ok=True)
    # Exclusive creation protects any pre-existing local example.
    with example.open("xb") as stream:
        stream.write(source.read_bytes())
    arguments = [args.cargo, "build", "-p", "grunkalunka-compiler", "--example",
                 "lowering_parity_snapshot", "--locked", "--offline", "--target-dir", str(target)]
    try:
        subprocess.run(arguments, cwd=repo, check=True)
        suffix = ".exe" if __import__("os").name == "nt" else ""
        built = target / "debug/examples" / ("lowering_parity_snapshot" + suffix)
        output.mkdir(parents=True, exist_ok=False)
        executable = output / ("parity-probe" + suffix)
        shutil.copy2(built, executable)
        receipt = {
            "source_commit": commit,
            "source_tree": command(["git", "rev-parse", "HEAD^{tree}"], repo),
            "command": arguments,
            "cargo_version": command([args.cargo, "--version"], repo),
            "rustc_version": command(["rustc", "-Vv"], repo),
            "cargo_lock_sha256": hashlib.sha256((repo / "Cargo.lock").read_bytes()).hexdigest(),
            "probe_source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            "binary_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
            "profile": "debug",
            "thread_stack_bytes": 2 * 1024 * 1024,
        }
        (output / "build-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n",
                                                  encoding="utf-8", newline="\n")
    finally:
        example.unlink()
    print(json.dumps({"probe": str(executable), "commit": commit}))


if __name__ == "__main__":
    main()
