#!/usr/bin/env python3
"""Serial, balanced fresh-process compiler observations; no performance gate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import resource
import statistics
import subprocess
import time


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def competing_jobs():
    found = []
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit():
            continue
        try:
            words = (proc / 'cmdline').read_bytes().split(b'\0')
            program = Path(os.fsdecode(words[0])).name
            if program in ('cargo', 'rustc', 'clippy-driver') or any(
                program.startswith(prefix) for prefix in
                ('async_suspension-', 'runtime_closures-', 'grunkalunka_compiler-')
            ):
                found.append({'pid': int(proc.name), 'program': program})
        except (FileNotFoundError, PermissionError):
            pass
    return found


def limits():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def summarize(records, iterations):
    table = []
    ids = sorted({r['case'] for r in records})
    for case in ids:
        selected = [r for r in records if r['case'] == case]
        revisions = {}
        for revision in ('baseline', 'candidate'):
            rs = [r for r in selected if r['revision'] == revision]
            times = [statistics.median(r['report']['compileBatchNs']) / iterations for r in rs]
            first = [r['report']['firstCompileNs'] for r in rs]
            peaks = [r['report']['processMemory']['afterBatches']['peakRssBytes'] for r in rs]
            releases = [statistics.median(r['report']['releaseBatchNs']) / iterations for r in rs]
            revisions[revision] = {
                'processes': len(rs), 'medianCompileNs': statistics.median(times),
                'minProcessMedianCompileNs': min(times), 'maxProcessMedianCompileNs': max(times),
                'medianFirstCompileNs': statistics.median(first),
                'medianReleaseNs': statistics.median(releases),
                'medianPeakRssBytes': statistics.median(peaks),
                'minPeakRssBytes': min(peaks), 'maxPeakRssBytes': max(peaks),
            }
        pairs = {}
        for r in selected:
            pairs.setdefault(r['repetition'], {})[r['revision']] = r
        ratios = [statistics.median(p['candidate']['report']['compileBatchNs']) /
                  statistics.median(p['baseline']['report']['compileBatchNs'])
                  for p in pairs.values()]
        table.append({'case': case, 'outcome': selected[0]['report']['outcome'],
                      'revisions': revisions, 'pairedCompileRatios': ratios,
                      'medianPairedCompileRatio': statistics.median(ratios)})
    return table


def main():
    p = argparse.ArgumentParser()
    p.add_argument('baseline', type=Path)
    p.add_argument('candidate', type=Path)
    p.add_argument('manifest', type=Path)
    p.add_argument('frozen', type=Path)
    p.add_argument('output', type=Path)
    p.add_argument('--repetitions', type=int, default=7)
    p.add_argument('--samples', type=int, default=5)
    p.add_argument('--iterations', type=int, default=5)
    p.add_argument('--warmup', type=int, default=3)
    args = p.parse_args()
    assert 3 <= args.repetitions <= 12
    assert 2 <= args.samples <= 12 and 1 <= args.iterations <= 32 and 0 <= args.warmup <= 32
    assert not competing_jobs(), 'wait for competing Rust builds/tests to stop before measurements'
    rows = [line.split('\t') for line in args.manifest.read_text().splitlines() if line]
    assert all(len(row) == 5 for row in rows)
    assert len({row[0] for row in rows}) == len(rows)
    args.output.mkdir(exist_ok=False)
    binaries = {'baseline': args.baseline.resolve(), 'candidate': args.candidate.resolve()}
    env = {'uname': list(os.uname()), 'affinity': sorted(os.sched_getaffinity(0)),
           'load_before': list(os.getloadavg()), 'cpuinfo': Path('/proc/cpuinfo').read_text(),
           'memory_info': Path('/proc/meminfo').read_text(),
           'cgroup_cpu_max': Path('/sys/fs/cgroup/cpu.max').read_text(),
           'cgroup_memory_max': Path('/sys/fs/cgroup/memory.max').read_text()}
    metadata = {'binaries': {k: {'path': str(v), 'sha256': sha256(v)} for k, v in binaries.items()},
                'manifest_sha256': sha256(args.manifest), 'configuration': vars(args).copy(),
                'environment': env, 'ordering': {'workload_shuffle_seed': '20261103 + repetition', 'revision_order': '(repetition + original_manifest_index) % 2; baseline first when zero', 'per_case_balance': 'four/three for seven repetitions'},
                'timing_scope': 'public compile API plus retained outcomes; validation and drops excluded; drops separately timed',
                'memory_scope': 'Linux process-lifetime VmHWM, including harness, thread, compiler, allocator, retained results, frozen-output validation; not isolated compiler allocations',
                'semantic_validation': 'Every first output matches frozen baseline bytes; all repeated outputs match first. No Wasmtime is linked or executed by this harness.'}
    metadata['configuration'] = {k: str(v) if isinstance(v, Path) else v for k, v in metadata['configuration'].items()}
    (args.output / 'metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')
    original_index = {row[0]: index for index, row in enumerate(rows)}
    records = []
    for repetition in range(args.repetitions):
        order = rows.copy()
        random.Random(20261103 + repetition).shuffle(order)
        for index, row in enumerate(order):
            revisions = ['baseline', 'candidate'] if (repetition + original_index[row[0]]) % 2 == 0 else ['candidate', 'baseline']
            for revision in revisions:
                competing = competing_jobs()
                if competing:
                    (args.output / 'interrupted.json').write_text(json.dumps(competing, indent=2) + '\n')
                    raise RuntimeError('competing build/test started; retain partial samples but do not summarize them as a complete run')
                command = [str(binaries[revision]), 'measure', str(args.manifest.resolve()), row[0],
                           str(args.frozen.resolve()), str(args.samples), str(args.iterations),
                           str(args.warmup), 'linux-proc-status']
                start = time.perf_counter_ns()
                result = subprocess.run(command, capture_output=True, text=True, timeout=180, preexec_fn=limits)
                wall_ns = time.perf_counter_ns() - start
                receipt = {'case': row[0], 'revision': revision, 'repetition': repetition,
                           'shuffled_index': index, 'original_manifest_index': original_index[row[0]], 'pair_order': revisions, 'command': command, 'processWallNs': wall_ns, 'returncode': result.returncode,
                           'stderr': result.stderr, 'load_after': list(os.getloadavg())}
                if result.returncode:
                    (args.output / 'failure.json').write_text(json.dumps(receipt, indent=2) + '\n')
                    raise RuntimeError(f'probe failed: {row[0]} {revision}')
                report = json.loads(result.stdout)
                assert report['case'] == row[0] and report['outcome'] == row[3]
                assert report['frozenOutputMatched'] and report['buildProfile'] == 'release'
                assert report['threadStackBytes'] == 2097152
                assert report['samples'] == args.samples and report['iterations'] == args.iterations
                assert report['warmup'] == args.warmup
                assert len(report['compileBatchNs']) == len(report['releaseBatchNs']) == args.samples
                receipt['report'] = report
                records.append(receipt)
                with (args.output / 'samples.jsonl').open('a') as stream:
                    stream.write(json.dumps(receipt) + '\n')
        print(json.dumps({'completed_repetitions': repetition + 1, 'processes': len(records)}), flush=True)
    summary = {'processes': len(records), 'cases': len(rows), 'repetitions': args.repetitions,
               'all_frozen_outputs_matched': True, 'environment_load_after': list(os.getloadavg()),
               'workloads': summarize(records, args.iterations),
               'limitations': ['One shared Linux host and one fixed selected workload; no cross-platform inference.',
                               'Fresh processes control allocator history but do not clear OS caches or fix CPU frequency.',
                               'Small timings and RSS deltas may be dominated by scheduling, page granularity, or allocator variation.',
                               'Ratios describe paired observations; no timing threshold or performance-equivalence claim is imposed.']}
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps({'complete': True, 'processes': len(records), 'cases': len(rows)}), flush=True)


if __name__ == '__main__':
    main()
