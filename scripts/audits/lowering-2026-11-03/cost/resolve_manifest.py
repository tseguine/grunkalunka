#!/usr/bin/env python3
"""Expand fixed manifest paths relative to the manifest, preserving measured Rust code."""
import argparse
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    manifest = args.manifest.resolve()
    rows, sources = [], []
    for line in manifest.read_text().splitlines():
        row = line.split("\t")
        assert len(row) == 5
        path = Path(row[4])
        source = path.resolve() if path.is_absolute() else (manifest.parent / path).resolve()
        data = source.read_bytes()
        row[4] = str(source)
        rows.append("\t".join(row))
        sources.append({"case": row[0], "profile": row[1], "language": row[2],
                        "expected": row[3], "source": str(source), "bytes": len(data),
                        "sha256": hashlib.sha256(data).hexdigest()})
    with args.output.open("x") as stream:
        stream.write("\n".join(rows) + "\n")
    receipt = {"input_manifest_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest(),
               "resolved_manifest_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(),
               "sources": sources}
    with args.output.with_suffix(args.output.suffix + ".sources.json").open("x") as stream:
        json.dump(receipt, stream, indent=2)
        stream.write("\n")
    print(json.dumps({"resolved_cases": len(rows), "output": str(args.output.resolve())}))


if __name__ == "__main__":
    main()
