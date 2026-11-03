#!/usr/bin/env python3
"""Snapshot existing independent baseline files. Never invokes a compiler."""
import argparse
import ast
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

CHUNK_LIMIT = 120 * 1024
BASELINE = "926a4aa8bcf7c6082a3d09d795a7e8cd7bfea91c"
FINAL = "4bef43c35814d7cb747254ad4d4fbac0dc9715dd"
PUBLISHED_FINAL = "67400f91882124e021f4bf30157930bb91acba31"
PROBE_SHA = "13f3c789bbd405fb550bdb79886449c32dff4c7ebb944e97977d68f223c9b0d7"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_record(path):
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": digest(data)}


def git(repo, *arguments):
    return subprocess.check_output(["git", *arguments], cwd=repo, text=True, encoding="utf-8").strip()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n",
                    encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--baseline-repo", type=Path, required=True)
    args = parser.parse_args()
    root, output, repo = args.workspace.resolve(), args.destination.resolve(), args.baseline_repo.resolve()
    if git(repo, "rev-parse", "HEAD") != BASELINE:
        raise ValueError("fixture origins must use the exact published baseline")
    if git(repo, "status", "--porcelain", "--untracked-files=no"):
        raise ValueError("baseline fixture checkout has tracked modifications")
    artifacts = root / "lowering-parity"
    verifier = artifacts / "verify_next.py"
    syntax = ast.parse(verifier.read_text(encoding="utf-8"))
    assignment = next(node for node in syntax.body if isinstance(node, ast.Assign)
                      and any(isinstance(target, ast.Name) and target.id == "MATRICES"
                              for target in node.targets))
    matrices = eval(compile(ast.Expression(assignment.value), str(verifier), "eval"),
                    {"__builtins__": {}}, {"ROOT": root, "ARTIFACTS": artifacts})
    (output / "cases").mkdir(parents=True, exist_ok=False)
    (output / "historical").mkdir()
    probe = root / "grunkalunka-baseline/crates/grunkalunka-compiler/examples/lowering_parity.rs"
    if digest(probe.read_bytes()) != PROBE_SHA:
        raise ValueError("historical parity harness changed")
    shutil.copyfile(probe, output / "probe.rs")
    shutil.copyfile(artifacts / "generate-corpus.mjs", output / "generate-corpus.mjs")
    shutil.copyfile(verifier, output / "historical/verify_next.py")
    shutil.copyfile(artifacts / "freeze_matrix.py", output / "historical/freeze_matrix.py")
    shutil.copyfile(root / "declaration-probes/generate.py", output / "historical/declaration-generate.py")

    # These receipts retain the original paths, commands, discovery outcomes and
    # probe identities. They are evidence, not current executable configuration.
    archive = []
    receipts = [artifacts / name for name in (
        "final-retirement-validation.json", "final-retirement-independent-receipt.json",
        "callbacks-validation.json", "callee-continuation-validation.json",
        "runtime-continuation-final-validation.json", "closure-child-preparation-final-validation.json",
        "expression-resumption-validation.json", "github-snapshot-through-462.json")]
    receipts += sorted(artifacts.glob("*-discovery/receipt.json"))
    for source in receipts:
        if not source.is_file():
            continue
        name = source.name if source.parent == artifacts else source.parent.name + "-receipt.json"
        data = source.read_bytes()
        if len(data) > CHUNK_LIMIT:
            chunks = []
            for index, offset in enumerate(range(0, len(data), CHUNK_LIMIT), 1):
                path = output / "historical" / f"{name}.part-{index:03d}"
                path.write_bytes(data[offset:offset + CHUNK_LIMIT])
                chunks.append({"path": path.relative_to(output).as_posix(), **file_record(path)})
            archive.append({"historical_path": str(source), "bytes": len(data),
                            "sha256": digest(data), "chunks": chunks})
        else:
            destination = output / "historical" / name
            destination.write_bytes(data)
            archive.append({"historical_path": str(source), "path": destination.relative_to(output).as_posix(),
                            **file_record(destination)})

    origin_paths = sorted(path for path in (repo / "tests/compatibility").rglob("*") if path.is_file())
    origins = [{"repository_path": path.relative_to(repo).as_posix(), **file_record(path)}
               for path in origin_paths]
    catalog = {"schema_version": 1,
               "source_snapshot": {"source_commit": FINAL, "published_commit": PUBLISHED_FINAL,
                                   "tree": "9ea19ddd1ff1dbd0d39c46216d13a22747130735"},
               "historical_workspace_root": str(root), "chunk_limit_bytes": CHUNK_LIMIT,
               "probe_source_sha256": PROBE_SHA, "matrices": []}
    totals = Counter()
    source_contents = {}
    source_paths_bytes = 0
    normalized_matrix_names = []
    discovery_matrix_names = []
    for name, manifest, baseline in matrices:
        rows = [line.split("\t") for line in manifest.read_text(encoding="utf-8").splitlines()
                if line and not line.startswith("#")]
        summary_data = (baseline / "summary.tsv").read_bytes()
        summaries = summary_data.decode().splitlines()
        if summaries[0] != "id\tprofile\tlanguage\texpected\tactual":
            raise ValueError(f"unexpected historical summary: {name}")
        summaries = [line.split("\t") for line in summaries[1:]]
        if len(rows) != len(summaries):
            raise ValueError(f"summary/manifest count: {name}")
        item = {"name": name, "cases": len(rows), "chunks": [],
                "historical_manifest_path": str(manifest),
                "historical_manifest": file_record(manifest),
                "historical_baseline_path": str(baseline),
                "historical_baseline_directory": baseline.name,
                "historical_summary": {"bytes": len(summary_data), "sha256": digest(summary_data)},
                "source_origin": "maintained-baseline-corpus" if name == "corpus" else "fixed-pre-conversion-inputs"}
        auxiliary = []
        for extra in (manifest.parent / "bodies.json", manifest.parent / "cases.json", manifest.parent / "generate.py"):
            if extra.is_file():
                auxiliary.append({"historical_path": str(extra), **file_record(extra)})
        item["historical_input_provenance_files"] = auxiliary
        cases = []
        canonical_summary = ["id\tprofile\tlanguage\texpected\tactual"]
        expected_files = {"summary.tsv"}
        for row, summary in zip(rows, summaries):
            if len(row) != 5 or len(summary) != 5 or row[:3] != summary[:3]:
                raise ValueError(f"historical case identity mismatch: {name}")
            case_id, profile, language, manifest_expected, source_path = row
            summary_expected, actual = summary[3:]
            if actual not in {"ok", "error"} or manifest_expected not in {actual, "observe"} or summary_expected not in {actual, "observe"}:
                raise ValueError(f"unfrozen or mismatched historical outcome: {name}/{case_id}")
            source_data = Path(source_path).read_bytes()
            source_hash = digest(source_data)
            source_contents[source_hash] = len(source_data)
            source_paths_bytes += len(source_data)
            source_text = source_data.decode("utf-8")
            if source_text.encode("utf-8") != source_data:
                raise ValueError(f"non-roundtripping UTF-8: {name}/{case_id}")
            outputs = {}
            for suffix in (("wat", "wasm") if actual == "ok" else ("error",)):
                path = baseline / f"{case_id}.{suffix}"
                outputs[suffix] = file_record(path)
                expected_files.add(path.name)
            cases.append({"id": case_id, "profile": profile, "language": language,
                          "expected": actual, "source": source_text,
                          "source_sha256": source_hash, "source_bytes": len(source_data),
                          "historical_source_path": source_path,
                          "historical_manifest_expected": manifest_expected,
                          "historical_summary_expected": summary_expected,
                          "outputs": outputs})
            totals[actual] += 1
            canonical_summary.append("\t".join((case_id, profile, language, actual, actual)))
        if {path.name for path in baseline.iterdir()} != expected_files:
            raise ValueError(f"unexpected historical output file set: {name}")
        canonical_bytes = ("\n".join(canonical_summary) + "\n").encode()
        item["canonical_summary_sha256"] = digest(canonical_bytes)
        item["artifact_and_diagnostic_files"] = len(expected_files) - 1
        item["historical_metadata_normalization"] = any(
            case["historical_summary_expected"] == "observe"
            and case["historical_manifest_expected"] != "observe" for case in cases)
        if item["historical_metadata_normalization"]:
            normalized_matrix_names.append(name)
        if any(case["historical_manifest_expected"] == "observe" for case in cases):
            discovery_matrix_names.append(name)
        directory = output / "cases" / name
        directory.mkdir()
        chunk = bytearray()
        count = 0

        def flush():
            nonlocal chunk, count
            if not count:
                return
            path = directory / f"part-{len(item['chunks']) + 1:03d}.jsonl"
            path.write_bytes(chunk)
            item["chunks"].append({"path": path.relative_to(output).as_posix(),
                                   "cases": count, "bytes": len(chunk), "sha256": digest(chunk)})
            chunk = bytearray()
            count = 0

        for case in cases:
            line = (json.dumps(case, ensure_ascii=False, separators=(",", ":")) + "\n").encode()
            if len(line) > CHUNK_LIMIT:
                raise ValueError(f"case exceeds bounded chunk size: {name}/{case['id']}")
            if len(chunk) + len(line) > CHUNK_LIMIT:
                flush()
            chunk.extend(line)
            count += 1
        flush()
        catalog["matrices"].append(item)
    catalog["counts"] = {"matrices": len(matrices), "cases": sum(totals.values()),
                         "outcomes": dict(totals), "artifact_and_diagnostic_files": totals["ok"] * 2 + totals["error"],
                         "metadata_files": len(matrices), "source_paths_bytes": source_paths_bytes,
                         "unique_source_contents": len(source_contents),
                         "unique_source_bytes": sum(source_contents.values())}
    if catalog["counts"]["cases"] != 7487 or totals != {"ok": 6033, "error": 1454}:
        raise ValueError("historical inventory changed")
    write_json(output / "catalog.json", catalog)
    provenance = {
        "schema_version": 1,
        "expected_output_policy": "Only pre-existing baseline files are read; no compiler is invoked and no candidate outputs supply expectations.",
        "source_generator": {"path": "generate-corpus.mjs", **file_record(output / "generate-corpus.mjs"),
                             "pinned_repository_commit": BASELINE,
                             "pinned_repository_tree": git(repo, "rev-parse", "HEAD^{tree}"),
                             "command": "node generate-corpus.mjs PINNED_BASELINE_CHECKOUT NEW_OUTPUT_DIRECTORY",
                             "entry_points": ["tests/compatibility/corpus.mjs", "tests/compatibility/program-sources.mjs",
                                              "tests/compatibility/primitive-values.mjs"],
                             "direct_fixture_overrides": ["tests/compatibility/fixtures/affine.js", "tests/compatibility/fixtures/affine.ts"]},
        "repository_origin_files": origins,
        "archived_receipts": archive,
        "historical_normalization_matrices": normalized_matrix_names,
        "historical_observe_manifest_matrices": discovery_matrix_names,
        "provenance_limits": [
            "Historical baselines came from several pre-conversion snapshots; a single original freeze revision is not asserted for all matrices.",
            "Where no standalone source generator was retained, complete fixed source bytes are the reproducible input authority.",
            "Original absolute paths in archived receipts identify historical files only; the portable runner does not depend on them.",
            "Source snapshots integrated onto different publication parents are distinguished from identical-tree mappings.",
        ],
    }
    write_json(output / "provenance.json", provenance)
    print(json.dumps(catalog["counts"], indent=2))


if __name__ == "__main__":
    main()
