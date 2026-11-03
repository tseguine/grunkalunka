// Read-only inspection of the existing planner graph for proposed bundle paths.
// This does not change ownership, run selected commands, or claim hosted CI success.
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(process.argv[2]);
const bundle = resolve(process.argv[3]);
const load = name => import(pathToFileURL(join(root, 'ci/impact', name)).href);
const { cargoModules, planImpact } = await load('impact.mjs');
const { cargoTestNodes } = await load('cargo-test-nodes.mjs');
const { checkNodes, verifyCanonicalChecks } = await load('canonical.mjs');
const { auditSourceSelection } = await load('source-dependency-audit.mjs');
const { auditBrowserWorkflows } = await load('external-workflows.mjs');
const catalog = folder => readdirSync(join(root, 'ci/impact', folder)).filter(name => name.endsWith('.json')).sort().map(name => JSON.parse(readFileSync(join(root, 'ci/impact', folder, name), 'utf8')));
const files = directory => readdirSync(directory, { withFileTypes: true }).flatMap(entry => entry.isDirectory() ? files(join(directory, entry.name)) : [join(directory, entry.name)]);
const paths = files(bundle).map(file => 'scripts/audits/lowering-2026-11-03/' + file.slice(bundle.length + 1)).sort();
const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps', '--offline'], { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }));
const packages = metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id));
const modules = [...cargoModules(metadata, root), ...catalog('owners')];
const groups = catalog('targets');
const declared = catalog('checks');
const cargoTests = cargoTestNodes(packages, catalog('cargo-tests'));
const tests = [...cargoTests, ...checkNodes(declared, packages.map(pkg => pkg.name), cargoTests.map(test => test.id))];
const source = readFileSync(join(root, 'ci/impact/plan.mjs'), 'utf8');
const literalPaths = name => JSON.parse(source.match(new RegExp(`const ${name} = (\\[[\\s\\S]*?\\]);`))[1].replaceAll("'", '"').replace(/,\s*\]/g, ']'));
const global = literalPaths('global');
const ignored = literalPaths('ignored');
const observed = JSON.parse(execFileSync(process.execPath, ['ci/impact/collect-checks.mjs'], { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }));
const sourceDependencyAudit = auditSourceSelection({ specs: catalog('source-audits'), readSource: file => readFileSync(join(root, file), 'utf8'), exists: file => existsSync(join(root, file)), planForPath: file => planImpact({ changedPaths: [file], modules, groups, tests, global, ignored }) });
const result = planImpact({ changedPaths: paths, modules, groups, tests, global, ignored });
console.log(JSON.stringify({ scope: 'Proposed bundle paths against unchanged final-source planner; no selected command execution.', sourceCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), ...result, canonicalCoverage: verifyCanonicalChecks(observed, declared), sourceDependencyAudit, externalWorkflows: auditBrowserWorkflows(paths, catalog('workflows'), file => readFileSync(join(root, file), 'utf8')) }, null, 2));
