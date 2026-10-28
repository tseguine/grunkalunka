import assert from 'node:assert/strict';
import test from 'node:test';
import { SAMPLE_CHANGES, validateLayoutSelection } from './validate-layout-selection.mjs';

function example() {
  return {
    impact: {
      full: false, reasons: [], changedPaths: [...SAMPLE_CHANGES],
      ignoredPaths: ['docs/headless-hit-test.md'],
      changedOwners: ['grunkalunka-layout'], build: ['grunkalunka-layout'],
      tests: ['cargo-test:grunkalunka-layout', 'check:workspace-clippy',
        'check:workspace-format', 'check:workspace-tests'],
    },
    preview: {
      advisoryOnly: true, mode: 'scoped-preview', reason: null,
      selectedBuildPackages: ['grunkalunka-layout'],
      selectedTestPackages: ['grunkalunka-layout'],
      commands: {
        format: ['cargo', 'fmt', '--all', '--check'],
        build: ['cargo', 'build', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout'],
        clippy: ['cargo', 'clippy', '--all-targets', '--locked', '--offline', '-j', '4',
          '-p', 'grunkalunka-layout', '--', '-D', 'warnings'],
        test: ['cargo', 'test', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout'],
      },
    },
  };
}

test('only the exact isolated layout selection can execute', () => {
  const { impact, preview } = example();
  const commands = validateLayoutSelection(impact, preview);
  assert.deepEqual(commands, preview.commands);
  commands.test.push('--help');
  assert.equal(preview.commands.test.includes('--help'), false);
});

test('full fallback must not silently turn into partial execution', () => {
  const { impact, preview } = example();
  impact.full = true;
  impact.reasons = ['unknown input'];
  assert.throws(() => validateLayoutSelection(impact, preview), /full fallback/);
});

test('changed paths or selected tests outside the layout slice fail closed', () => {
  for (const mutation of [
    x => x.impact.changedPaths.push('crates/grunkalunka-runtime/src/lib.rs'),
    x => x.impact.changedOwners.push('grunkalunka-runtime'),
    x => x.impact.tests.push('check:compiler-compatibility-replay'),
    x => x.impact.tests.pop(),
    x => x.impact.build.push('grunkalunka-compiler'),
    x => x.preview.selectedBuildPackages.push('grunkalunka-runtime'),
    x => x.preview.selectedTestPackages.push('grunkalunka-runtime'),
  ]) {
    const inputs = example();
    mutation(inputs);
    assert.throws(() => validateLayoutSelection(inputs.impact, inputs.preview));
  }
});

test('missing, reordered, or expanded executable commands fail closed', () => {
  for (const mutation of [
    x => { x.preview.mode = 'full'; },
    x => { x.preview.advisoryOnly = false; },
    x => { x.preview.commands.test = null; },
    x => { x.preview.commands.build.pop(); },
    x => { x.preview.commands.clippy.push('-p', 'grunkalunka-runtime'); },
    x => { x.preview.commands.build[0] = 'sh'; },
    x => { x.preview.commands.format.push('--verbose'); },
  ]) {
    const inputs = example();
    mutation(inputs);
    assert.throws(() => validateLayoutSelection(inputs.impact, inputs.preview));
  }
});
