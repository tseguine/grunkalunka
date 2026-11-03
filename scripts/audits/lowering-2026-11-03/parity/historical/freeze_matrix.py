#!/usr/bin/env python3
"""Observe each new case in an isolated probe process, then freeze expectations."""
import argparse
import collections
import hashlib
import json
import resource
import shutil
import subprocess
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('manifest', type=Path)
p.add_argument('probe', type=Path)
p.add_argument('baseline', type=Path)
p.add_argument('discovery', type=Path)
args = p.parse_args()
original = args.manifest.read_text()
rows = [line.split('\t') for line in original.splitlines() if line and not line.startswith('#')]
assert all(len(row) == 5 and row[3] == 'observe' for row in rows)
assert len({row[0] for row in rows}) == len(rows)
args.baseline.mkdir(exist_ok=False)
args.discovery.mkdir(exist_ok=False)
summary = ['id\tprofile\tlanguage\texpected\tactual']
receipt = {'probe': str(args.probe), 'probe_sha256': hashlib.sha256(args.probe.read_bytes()).hexdigest(), 'cases': []}

def child_limits():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))

for row in rows:
    case_id = row[0]
    case_dir = args.discovery / case_id
    case_dir.mkdir()
    manifest = case_dir / 'input.tsv'
    manifest.write_text('\t'.join(row) + '\n')
    output = case_dir / 'output'
    run = subprocess.run([str(args.probe), str(manifest), str(output)], capture_output=True,
                         text=True, timeout=60, preexec_fn=child_limits)
    item = {'id': case_id, 'status': run.returncode, 'stdout': run.stdout, 'stderr': run.stderr}
    receipt['cases'].append(item)
    (args.discovery / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    assert run.returncode == 0, item
    lines = (output / 'summary.tsv').read_text().splitlines()
    assert len(lines) == 2
    result = lines[1].split('\t')
    assert result[:4] == row[:4] and result[4] in ('ok', 'error'), result
    row[3] = result[4]
    result[3] = result[4]
    summary.append('\t'.join(result))
    for artifact in output.iterdir():
        if artifact.name != 'summary.tsv':
            shutil.copyfile(artifact, args.baseline / artifact.name)

(args.discovery / 'manifest-observe.tsv').write_text(original)
args.manifest.write_text(''.join('\t'.join(row) + '\n' for row in rows))
(args.baseline / 'summary.tsv').write_text('\n'.join(summary) + '\n')
print(json.dumps({'cases': len(rows), 'counts': dict(collections.Counter(row[3] for row in rows)),
                  'baseline': str(args.baseline), 'probe_sha256': receipt['probe_sha256']}))
