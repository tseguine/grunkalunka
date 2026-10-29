// One-command A1 development build using the existing Cargo workspace.
// No hidden dependency fetch, output overwrite, packaging or CI workflow.
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));

export const buildCommands = Object.freeze([
  ['build', '-p', 'grunkalunka-cli', '--locked', '--offline'],
  ['build', '-p', 'grunkalunka-markdown', '--example', 'outline_guest', '--locked', '--offline'],
  ['build', '-p', 'grunkalunka-markdown', '--example', 'outline_guest',
    '--target', 'wasm32-unknown-unknown', '--locked', '--offline'],
  ['build', '-p', 'grunkalunka-runtime', '--example', 'a1_note_preview', '--locked', '--offline'],
  ['build', '-p', 'grunkalunka-layout', '--example', 'a1_note_scene', '--locked', '--offline'],
]);

export function buildNotePreview() {
  for (const args of buildCommands) {
    const result = spawnSync('cargo', args, {
      cwd: root, shell: false, stdio: 'inherit', timeout: 10 * 60_000,
      killSignal: 'SIGKILL',
    });
    if (result.error || result.signal || result.status !== 0) {
      throw new Error('A1 build failed for cargo ' + args.join(' ') + ': ' +
        (result.error?.message || result.signal || result.status));
    }
  }
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    if (process.argv.length !== 2) throw new Error('usage: node examples/note-preview/build.mjs');
    buildNotePreview();
    console.log('PASS A1 development build (host binaries and Markdown Wasm guest)');
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
