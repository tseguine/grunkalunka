#!/usr/bin/env python3
"""Run immutable compiler probes and compare fresh outputs to frozen baselines."""
import argparse
import filecmp
import hashlib
import json
from pathlib import Path
import subprocess
import time

ROOT = Path('/workspace/scratch/88caffc5ffb1')
ARTIFACTS = ROOT / 'lowering-parity'
MATRICES = [
    ('corpus', ARTIFACTS / 'corpus/manifest.tsv', ARTIFACTS / 'baseline'),
    ('factory', ROOT / 'factory-probes/matrix.tsv', ARTIFACTS / 'baseline-factory'),
    ('factory-forwarded', ROOT / 'factory-probes/forwarded.tsv', ARTIFACTS / 'baseline-factory-forwarded'),
    ('factory-effects', ROOT / 'factory-effect-probes/matrix.tsv', ARTIFACTS / 'baseline-factory-effects'),
    ('factory-effect-callbacks', ROOT / 'factory-effect-probes/callback-depth.tsv', ARTIFACTS / 'baseline-factory-effect-callbacks'),
    ('callback-calls', ROOT / 'callback-call-probes/matrix.tsv', ARTIFACTS / 'baseline-callback-calls'),
    ('callback-call-limits', ROOT / 'callback-call-probes/negative-depth.tsv', ARTIFACTS / 'baseline-callback-call-limits'),
    ('generic-callees', ROOT / 'callee-baseline-probes/matrix.tsv', ARTIFACTS / 'baseline-generic-callees'),
    ('callee-extra', ROOT / 'callee-extra-probes/matrix.tsv', ARTIFACTS / 'baseline-callee-extra'),
    ('concise-factories', ROOT / 'concise-baseline-probes/matrix.tsv', ARTIFACTS / 'baseline-concise-factories'),
    ('concise-combined', ROOT / 'concise-combined-probes/matrix.tsv', ARTIFACTS / 'baseline-concise-combined'),
    ('charged-concise', ROOT / 'charged-concise-probes/matrix.tsv', ARTIFACTS / 'baseline-charged-concise'),
    ('charged-boundaries', ROOT / 'charged-boundary-probes/matrix.tsv', ARTIFACTS / 'baseline-charged-boundaries'),
    ('runtime-preparation', ROOT / 'runtime-preparation-probes/matrix.tsv', ARTIFACTS / 'baseline-runtime-preparation'),
    ('runtime-continuation', ROOT / 'runtime-continuation-probes/matrix.tsv', ARTIFACTS / 'baseline-runtime-continuation'),
    ('runtime-managed', ROOT / 'runtime-managed-probes/matrix.tsv', ARTIFACTS / 'baseline-runtime-managed'),
    ('runtime-order', ROOT / 'runtime-order-probes/matrix.tsv', ARTIFACTS / 'baseline-runtime-order'),
    ('declarations', ROOT / 'declaration-probes/matrix.tsv', ARTIFACTS / 'baseline-declarations'),
    ('declaration-boundaries', ROOT / 'declaration-boundary-probes/matrix.tsv', ARTIFACTS / 'baseline-declaration-boundaries'),
    ('block-factory-call', ROOT / 'block-factory-call-probes/matrix.tsv', ARTIFACTS / 'baseline-block-factory-call'),
    ('closure-checking', ROOT / 'closure-checking-probes/matrix.tsv', ARTIFACTS / 'baseline-closure-checking'),
    ('optional-factory', ROOT / 'optional-factory-probes/matrix.tsv', ARTIFACTS / 'baseline-optional-factory'),
    ('unchecked-closure', ROOT / 'unchecked-closure-probes/matrix.tsv', ARTIFACTS / 'baseline-unchecked-closure'),
    ('unchecked-closure-depth', ROOT / 'unchecked-closure-depth-fast-probes/matrix.tsv', ARTIFACTS / 'baseline-unchecked-closure-depth-fast'),
    ('unchecked-closure-extra', ROOT / 'unchecked-closure-extra-probes/matrix.tsv', ARTIFACTS / 'baseline-unchecked-closure-extra'),
    ('for-declarations', ROOT / 'for-declaration-probes/matrix.tsv', ARTIFACTS / 'baseline-for-declarations'),
    ('for-declaration-boundaries', ROOT / 'for-declaration-boundary-probes/matrix.tsv', ARTIFACTS / 'baseline-for-declaration-boundaries'),
    ('const-metadata', ROOT / 'const-metadata-probes/matrix.tsv', ARTIFACTS / 'baseline-const-metadata'),
    ('const-metadata-managed', ROOT / 'const-metadata-managed-probes/matrix.tsv', ARTIFACTS / 'baseline-const-metadata-managed'),
    ('declaration-checks', ROOT / 'declaration-check-probes/matrix.tsv', ARTIFACTS / 'baseline-declaration-checks'),
    ('const-metadata-zero', ROOT / 'const-metadata-zero-probes/matrix.tsv', ARTIFACTS / 'baseline-const-metadata-zero'),
    ('materialization', ROOT / 'materialization-probes/matrix.tsv', ARTIFACTS / 'baseline-materialization'),
    ('factory-check-continuation', ROOT / 'factory-check-continuation-probes/matrix.tsv', ARTIFACTS / 'baseline-factory-check-continuation'),
    ('expression-resumption', ROOT / 'expression-resumption-probes/matrix.tsv', ARTIFACTS / 'baseline-expression-resumption'),
]

parser = argparse.ArgumentParser()
parser.add_argument('label')
parser.add_argument('commit')
parser.add_argument('probe', type=Path)
parser.add_argument('--only', nargs='+')
args = parser.parse_args()
report = {
    'commit': args.commit,
    'probe': str(args.probe),
    'probe_sha256': hashlib.sha256(args.probe.read_bytes()).hexdigest(),
    'matrices': [],
}
failed = False
for name, manifest, baseline in MATRICES:
    if args.only and name not in args.only:
        continue
    rows = [line.split('\t') for line in manifest.read_text().splitlines()
            if line and not line.startswith('#')]
    assert all(len(row) == 5 for row in rows), name
    assert len({row[0] for row in rows}) == len(rows), name
    assert all(Path(row[4]).is_file() for row in rows), name
    output = ARTIFACTS / (args.label + '-' + name)
    output.mkdir(exist_ok=False)
    started = time.monotonic()
    result = subprocess.run([str(args.probe), str(manifest), str(output)],
                            text=True, capture_output=True)
    expected_files = {p.name for p in baseline.iterdir() if p.is_file()}
    actual_files = {p.name for p in output.iterdir() if p.is_file()}
    missing = sorted(expected_files - actual_files)
    extra = sorted(actual_files - expected_files)
    changed = sorted(name for name in expected_files & actual_files
                     if not filecmp.cmp(baseline / name, output / name, shallow=False))
    normalized_metadata = []
    if 'summary.tsv' in changed:
        # Some independently frozen matrices retain discovery's expected=observe.
        # Normalize only that metadata column, after checking its actual status
        # exactly matches the now-frozen manifest. All artifacts remain byte exact.
        summary = [line.split('\t') for line in (baseline / 'summary.tsv').read_text().splitlines()]
        frozen = {row[0]: row for row in rows}
        valid = summary[0] == ['id', 'profile', 'language', 'expected', 'actual']
        valid &= len(summary) == len(rows) + 1
        if valid:
            for row in summary[1:]:
                expected = frozen.get(row[0])
                if len(row) != 5 or expected is None or row[:3] != expected[:3] or row[4] != expected[3]:
                    valid = False
                    break
                if row[3] == 'observe':
                    row[3] = expected[3]
                elif row[3] != expected[3]:
                    valid = False
                    break
        canonical = '\n'.join('\t'.join(row) for row in summary) + '\n'
        if valid and canonical == (output / 'summary.tsv').read_text():
            changed.remove('summary.tsv')
            normalized_metadata.append('summary.tsv: expected=observe replaced by verified frozen manifest expectation')
    item = {
        'matrix': name, 'cases': len(rows), 'baseline': str(baseline),
        'output': str(output), 'process_status': result.returncode,
        'stdout': result.stdout.strip(), 'stderr': result.stderr.strip(),
        'compared_files': len(expected_files & actual_files),
        'missing': missing, 'extra': extra, 'changed': changed,
        'normalized_metadata': normalized_metadata,
        'seconds': round(time.monotonic() - started, 3),
    }
    report['matrices'].append(item)
    print(json.dumps(item), flush=True)
    failed |= result.returncode != 0 or bool(missing or extra or changed)
    (ARTIFACTS / (args.label + '-validation.json')).write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'cases': sum(x['cases'] for x in report['matrices']), 'failed': failed}), flush=True)
raise SystemExit(1 if failed else 0)
