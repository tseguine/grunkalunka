#!/usr/bin/env python3
"""Compare a regenerated output directory with the frozen artifact-hash receipt."""
import argparse
import hashlib
import json
from pathlib import Path


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()
    expected = json.loads(args.receipt.read_text())["artifacts"]
    actual = {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
              for path in args.directory.iterdir() if path.is_file()}
    difference = sorted(name for name in expected.keys() | actual.keys()
                        if expected.get(name) != actual.get(name))
    assert not difference, difference
    print(json.dumps({"files": len(actual), "all_frozen_hashes_match": True}))
