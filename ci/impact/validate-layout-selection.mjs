// Executable-scoping safety gate for one bounded layout-only scenario.
// This is NOT a general partial-PR executor and never gates canonical CI.
import assert from 'node:assert/strict';

// Actual changed-file set of merged PR #75, replayed against current source.
export const SAMPLE_CHANGES = Object.freeze([
  'crates/grunkalunka-layout/examples/click_reflow_ppm.rs',
  'crates/grunkalunka-layout/tests/headless_input_reflow.rs',
  'docs/headless-hit-test.md',
]);
const LAYOUT = 'grunkalunka-layout';
const expectedTargets = [
  'cargo-test:grunkalunka-layout',
  'check:workspace-clippy',
  'check:workspace-format',
  'check:workspace-tests',
];
const commands = {
  format: ['cargo', 'fmt', '--all', '--check'],
  build: ['cargo', 'build', '--locked', '--offline', '-j', '4', '-p', LAYOUT],
  clippy: ['cargo', 'clippy', '--all-targets', '--locked', '--offline', '-j', '4',
    '-p', LAYOUT, '--', '-D', 'warnings'],
  test: ['cargo', 'test', '--locked', '--offline', '-j', '4', '-p', LAYOUT],
};

export function validateLayoutSelection(impact, preview) {
  assert.ok(impact && preview, 'missing advisory inputs');
  assert.equal(impact.full, false, 'full fallback: ' + (impact.reasons ?? []).join('; '));
  assert.deepEqual(impact.changedPaths, [...SAMPLE_CHANGES]);
  assert.deepEqual(impact.ignoredPaths, ['docs/headless-hit-test.md']);
  assert.deepEqual(impact.changedOwners, [LAYOUT]);
  assert.deepEqual(impact.build, [LAYOUT],
    'a layout-only change must not require unrelated workspace crates');
  assert.deepEqual(impact.tests, expectedTargets);
  assert.equal(preview.mode, 'scoped-preview', preview.reason ?? 'unsafe Cargo scope');
  assert.equal(preview.advisoryOnly, true);
  assert.deepEqual(preview.selectedBuildPackages, [LAYOUT]);
  assert.deepEqual(preview.selectedTestPackages, [LAYOUT]);
  assert.deepEqual(preview.commands, commands,
    'the executable sample only permits the pinned, expected four Cargo commands');
  return structuredClone(commands);
}
