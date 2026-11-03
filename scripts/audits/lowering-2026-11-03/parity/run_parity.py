#!/usr/bin/env python3
"""Replay fixed lowering inputs against independent, frozen output hashes."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import time

BUNDLE = Path(__file__).resolve().parent
HEADER = "id\tprofile\tlanguage\texpected\tactual\n"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def file_sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def bundled_path(relative):
    path = PurePosixPath(relative)
    require(not path.is_absolute() and ".." not in path.parts,
            f"invalid bundled path: {relative}")
    resolved = (BUNDLE / path).resolve()
    require(resolved.is_relative_to(BUNDLE), f"path escapes bundle: {relative}")
    return resolved


def read_catalog(selected=None):
    catalog = json.loads((BUNDLE / "catalog.json").read_text(encoding="utf-8"))
    require(catalog["schema_version"] == 1, "unsupported catalog schema")
    names = [matrix["name"] for matrix in catalog["matrices"]]
    require(len(names) == len(set(names)), "duplicate matrix name")
    if selected:
        require(set(selected) <= set(names), f"unknown matrices: {set(selected) - set(names)}")
    result = []
    for matrix in catalog["matrices"]:
        for key in ("name", "historical_baseline_directory"):
            require(re.fullmatch(r"[A-Za-z0-9_-]+", matrix[key]) is not None,
                    f"invalid matrix path component: {matrix[key]}")
        if selected and matrix["name"] not in selected:
            continue
        cases = []
        for chunk in matrix["chunks"]:
            data = bundled_path(chunk["path"]).read_bytes()
            require(len(data) == chunk["bytes"] and sha256(data) == chunk["sha256"],
                    f"changed case chunk: {chunk['path']}")
            # JSONL uses literal LF delimiters. U+2028/U+2029 inside source
            # strings are valid JSON and must not become record boundaries.
            rows = [json.loads(line) for line in data.decode("utf-8").split("\n") if line]
            require(len(rows) == chunk["cases"], f"wrong chunk count: {chunk['path']}")
            cases.extend(rows)
        require(len(cases) == matrix["cases"], f"wrong case count: {matrix['name']}")
        require(len({case["id"] for case in cases}) == len(cases),
                f"duplicate case ID in {matrix['name']}")
        for case in cases:
            require(re.fullmatch(r"[A-Za-z0-9_-]+", case["id"]) is not None,
                    f"invalid case ID: {case['id']}")
            require(case["profile"] in {"numeric", "value", "async", "heap", "ui"},
                    f"unknown profile: {case['id']}")
            require(case["language"] in {"js", "ts"}, f"unknown language: {case['id']}")
            require(case["expected"] in {"ok", "error"}, f"unfrozen outcome: {case['id']}")
            source = case["source"].encode("utf-8")
            require(len(source) == case["source_bytes"] and sha256(source) == case["source_sha256"],
                    f"changed source: {case['id']}")
            suffixes = {"wat", "wasm"} if case["expected"] == "ok" else {"error"}
            require(set(case["outputs"]) == suffixes, f"wrong output kinds: {case['id']}")
            for output in case["outputs"].values():
                require(isinstance(output["bytes"], int) and output["bytes"] >= 0,
                        f"invalid output length: {case['id']}")
                require(re.fullmatch(r"[a-f0-9]{64}", output["sha256"]) is not None,
                        f"invalid output hash: {case['id']}")
            for key in ("historical_manifest_expected", "historical_summary_expected"):
                require(case[key] in {case["expected"], "observe"},
                        f"inconsistent historical expectation: {case['id']}")
        result.append((matrix, cases))
    return catalog, result


def canonical_summary(cases):
    return HEADER + "".join(
        "\t".join((case["id"], case["profile"], case["language"],
                   case["expected"], case["expected"])) + "\n"
        for case in cases)


def materialize(destination, matrices):
    destination = destination.resolve()
    require(not any(char in str(destination) for char in "\t\r\n"),
            "output path cannot contain TSV delimiters")
    destination.mkdir(parents=True, exist_ok=False)
    manifests = {}
    for matrix, cases in matrices:
        directory = destination / matrix["name"]
        directory.mkdir()
        rows = []
        for case in cases:
            source = directory / f"{case['id']}.{case['language']}"
            source.write_bytes(case["source"].encode("utf-8"))
            # Admission is now explicit even when discovery used observe.
            rows.append("\t".join((case["id"], case["profile"], case["language"],
                                   case["expected"], str(source))))
        manifest = directory / "manifest.tsv"
        manifest.write_text("\n".join(rows) + "\n", encoding="utf-8", newline="\n")
        manifests[matrix["name"]] = manifest
    return manifests


def verify_output(directory, matrix, cases, allow_historical_observe=False):
    require(directory.is_dir(), f"missing output directory: {directory}")
    expected_files = {"summary.tsv"} | {
        f"{case['id']}.{suffix}" for case in cases for suffix in case["outputs"]}
    actual_files = {path.name for path in directory.iterdir()}
    require(actual_files == expected_files,
            f"{matrix['name']}: missing={sorted(expected_files - actual_files)}, "
            f"extra={sorted(actual_files - expected_files)}")
    require(all((directory / name).is_file() for name in actual_files),
            f"non-file output in {directory}")
    raw_summary = (directory / "summary.tsv").read_bytes().decode("utf-8")
    rows = raw_summary.split("\n")
    require(rows and rows[0] + "\n" == HEADER and rows[-1] == ""
            and len(rows) == len(cases) + 2,
            f"invalid summary header/count: {matrix['name']}")
    normalized = []
    canonical_rows = [rows[0]]
    for case, line in zip(cases, rows[1:-1]):
        fields = line.split("\t")
        identity = [case["id"], case["profile"], case["language"]]
        require(len(fields) == 5 and fields[:3] == identity and fields[4] == case["expected"],
                f"changed summary identity/status: {matrix['name']}/{case['id']}")
        if fields[3] != case["expected"]:
            require(allow_historical_observe and fields[3] == "observe"
                    and "observe" in (case["historical_manifest_expected"],
                                       case["historical_summary_expected"]),
                    f"unexpected summary expectation: {matrix['name']}/{case['id']}")
            # Only this column changes, after independently checking actual.
            normalized.append(case["id"])
            fields[3] = case["expected"]
        canonical_rows.append("\t".join(fields))
        for suffix, expected in case["outputs"].items():
            path = directory / f"{case['id']}.{suffix}"
            require(path.stat().st_size == expected["bytes"]
                    and file_sha256(path) == expected["sha256"],
                    f"changed frozen output: {matrix['name']}/{path.name}")
    canonical = canonical_summary(cases).encode()
    require(("\n".join(canonical_rows) + "\n").encode() == canonical,
            f"non-expectation summary difference: {matrix['name']}")
    require(sha256(canonical) == matrix["canonical_summary_sha256"],
            f"changed canonical summary catalog: {matrix['name']}")
    return {"matrix": matrix["name"], "cases": len(cases),
            "compared_output_files": len(expected_files),
            "artifact_and_diagnostic_files": len(expected_files) - 1,
            "normalized_observe_case_ids": normalized, "all_match": True}


def equal_bytes(left, right):
    with left.open("rb") as a, right.open("rb") as b:
        while True:
            x, y = a.read(1024 * 1024), b.read(1024 * 1024)
            if x != y:
                return False
            if not x:
                return True


def run_probes(args, matrices):
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    manifests = materialize(args.output / "inputs", matrices)
    probes = {"candidate": args.probe.resolve()} if args.command == "check" else {
        "baseline": args.baseline_probe.resolve(), "candidate": args.candidate_probe.resolve()}
    report = {"schema_version": 1, "mode": args.command,
              "catalog_sha256": file_sha256(BUNDLE / "catalog.json"),
              "probe_source_sha256": file_sha256(BUNDLE / "probe.rs"),
              "probes": {name: {"path": str(path), "sha256": file_sha256(path)}
                         for name, path in probes.items()},
              "matrices": [], "complete": False, "all_match": False}
    outcomes = Counter(case["expected"] for _, cases in matrices for case in cases)
    report.update({"cases": sum(outcomes.values()), "outcomes": dict(outcomes),
                   "artifact_and_diagnostic_files": outcomes["ok"] * 2 + outcomes["error"],
                   "metadata_files": len(matrices)})
    receipt = args.output / "receipt.json"
    receipt.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    try:
        for matrix, cases in matrices:
            item = {"matrix": matrix["name"], "runs": {}}
            report["matrices"].append(item)
            for revision, probe in probes.items():
                output = args.output / revision / matrix["name"]
                output.parent.mkdir(exist_ok=True)
                command = [str(probe), str(manifests[matrix["name"]]), str(output)]
                started = time.monotonic()
                result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8",
                                        timeout=args.timeout)
                run = {"command": command, "returncode": result.returncode,
                       "stdout": result.stdout, "stderr": result.stderr,
                       "seconds": time.monotonic() - started}
                item["runs"][revision] = run
                require(result.returncode == 0,
                        f"probe failed: {revision}/{matrix['name']}: {result.stderr}")
                run["verification"] = verify_output(output, matrix, cases)
            if args.command == "compare":
                # Compare actual artifact bytes as well as their independent hashes.
                for case in cases:
                    for suffix in case["outputs"]:
                        name = f"{case['id']}.{suffix}"
                        require(equal_bytes(args.output / "baseline" / matrix["name"] / name,
                                            args.output / "candidate" / matrix["name"] / name),
                                f"baseline/candidate byte difference: {matrix['name']}/{name}")
                item["baseline_candidate_bytes_equal"] = True
            receipt.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
            print(json.dumps({"matrix": matrix["name"], "cases": len(cases), "all_match": True}), flush=True)
        report["complete"] = report["all_match"] = True
    except Exception as error:
        report["failure"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        receipt.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("inventory", "materialize", "check", "compare", "verify-existing"):
        command = commands.add_parser(name)
        command.add_argument("--only", nargs="+", help="matrix names; default is all 34")
        if name == "materialize":
            command.add_argument("output", type=Path)
        if name in ("check", "compare"):
            command.add_argument("--output", type=Path, required=True)
            command.add_argument("--timeout", type=float, default=900,
                                 help="seconds per matrix; preserves partial receipts on failure")
        if name == "check":
            command.add_argument("--probe", type=Path, required=True)
        if name == "compare":
            command.add_argument("--baseline-probe", type=Path, required=True)
            command.add_argument("--candidate-probe", type=Path, required=True)
        if name == "verify-existing":
            command.add_argument("output", type=Path)
            command.add_argument("--historical-layout", action="store_true",
                                 help="use recorded baseline directory names and allow recorded observe metadata")
    args = parser.parse_args()
    catalog, matrices = read_catalog(args.only)
    if args.command == "materialize":
        materialize(args.output, matrices)
        print(json.dumps({"output": str(args.output.resolve()), "cases": sum(len(c) for _, c in matrices)}))
    elif args.command in ("check", "compare"):
        report = run_probes(args, matrices)
        print(json.dumps({"complete": report["complete"], "all_match": report["all_match"]}))
    elif args.command == "verify-existing":
        reports = []
        for matrix, cases in matrices:
            name = matrix["historical_baseline_directory"] if args.historical_layout else matrix["name"]
            reports.append(verify_output(args.output / name, matrix, cases, args.historical_layout))
        print(json.dumps({"matrices": reports,
                          "cases": sum(row["cases"] for row in reports),
                          "compared_output_files": sum(row["compared_output_files"] for row in reports),
                          "all_match": True}, indent=2))
    else:
        counts = Counter(case["expected"] for _, cases in matrices for case in cases)
        print(json.dumps({"matrices": len(matrices), "cases": sum(counts.values()),
                          "outcomes": dict(counts),
                          "artifact_and_diagnostic_files": counts["ok"] * 2 + counts["error"],
                          "metadata_files": len(matrices),
                          "catalog_source": catalog["source_snapshot"]}, indent=2))


if __name__ == "__main__":
    main()
