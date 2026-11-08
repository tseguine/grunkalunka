// Independently discover literal Node/Rust source inputs, then verify test selection.
import { posix as path } from 'node:path';

const js = /\.(?:[cm]?js|[cm]?ts)$/;
const rust = /\.rs$/;
const imports = /\bimport\s+(?:[\s\S]{0,150}?\s+from\s*)?(['"])([^'"]+)\1/g;
// A barrel module changes when the modules it re-exports change, even if it
// contains no ordinary import declaration. Include named, star, namespace and
// TypeScript type-only re-export sources in the same dependency graph.
const reexports = /\bexport\s+(?:type\s+)?(?:\*\s*(?:as\s+[$\w]+\s*)?|\{[^}]*\})\s+from\s*(['"])([^'"]+)\1/g;
const importCalls = /\bimport\s*\(\s*(['"])([^'"]+)\1\s*\)/g;
// CommonJS modules and fixtures are also source inputs. Restrict this to
// bare require() calls rather than unrelated object.require() methods.
const requires = /(?<![.\w$])require\s*\(\s*(['"])([^'"]+)\1\s*\)/g;
const bareRequires = /(?<![.\w$])require\s*\(/g;
const urls = /\bnew\s+URL\s*\(\s*(['"])([^'"]+)\1\s*,\s*import\.meta\.url\s*,?\s*\)/g;
const urlTemplates = /\bnew\s+URL\s*\(\s*\x60([^\x60]*)\x60\s*,\s*import\.meta\.url\s*,?\s*\)/g;
const includes = /\binclude_(?:str|bytes)!\s*\(\s*"([^"]+)"\s*\)/g;
// include! splices Rust source text at compile time; omitting it can also omit
// every transitive input introduced by that text from the source audit.
const codeIncludes = /\binclude!\s*\(\s*"([^"]+)"\s*\)/g;
const rustPaths = /#\[\s*path\s*=\s*"([^"]+)"\s*\]/g;

function resolveSourceInput(source, specifier) {
  if (!specifier.startsWith('./') && !specifier.startsWith('../')) {
    throw new Error('not a relative source path: ' + specifier);
  }
  const target = path.normalize(path.join(path.dirname(source), specifier));
  if (target === '..' || target.startsWith('../') || path.isAbsolute(target)) {
    throw new Error('dependency escapes repository: ' + source + ' -> ' + specifier);
  }
  return target;
}

export function discoverLiteralSourceInputs(source, text) {
  if (typeof source !== 'string' || typeof text !== 'string' ||
      !js.test(source) && !rust.test(source)) {
    throw new Error('unsupported source scan');
  }
  const inputs = new Set(), unresolved = [];
  const add = name => {
    if (name.startsWith('./') || name.startsWith('../')) {
      inputs.add(resolveSourceInput(source, name));
    } else if (name.startsWith('/') || name.startsWith('file:')) {
      unresolved.push('non-relative file input: ' + name);
    }
  };
  if (js.test(source)) {
    for (const match of text.matchAll(imports)) add(match[2]);
    for (const match of text.matchAll(reexports)) add(match[2]);
    for (const match of text.matchAll(importCalls)) add(match[2]);
    const literalRequires = [...text.matchAll(requires)];
    for (const match of literalRequires) add(match[2]);
    if ([...text.matchAll(bareRequires)].length !== literalRequires.length) {
      unresolved.push('computed require expression: ' + source);
    }
    for (const match of text.matchAll(urls)) add(match[2]);
    for (const match of text.matchAll(urlTemplates)) {
      const name = match[1];
      // Generated compiler binaries below target/ are not Git-tracked inputs.
      if (name.startsWith('../../target/')) continue;
      if (name.includes('$' + '{')) unresolved.push('computed source URL: ' + source);
      else add(name);
    }
    if (/\bimport\s*\(\s*(?!['"])/.test(text)) {
      unresolved.push('computed import expression: ' + source);
    }
    for (const match of text.matchAll(/\bnew\s+URL\s*\(\s*([^\s])/g)) {
      if (['\'', '"', String.fromCharCode(96)].includes(match[1])) continue;
      if (!/^new\s+URL\s*\(\s*import\.meta\.url\s*,?\s*\)/.test(text.slice(match.index))) {
        unresolved.push('computed URL input: ' + source);
      }
    }
  } else {
    const literalIncludes = [...text.matchAll(includes), ...text.matchAll(codeIncludes)];
    for (const match of literalIncludes) add(match[1]);
    if ([...text.matchAll(/\binclude(?:_(?:str|bytes))?!\s*\(/g)].length !== literalIncludes.length) {
      unresolved.push('nonliteral Rust include: ' + source);
    }
    const literalPaths = [...text.matchAll(rustPaths)];
    for (const match of literalPaths) add(match[1]);
    if ([...text.matchAll(/#\[\s*path\s*=/g)].length !== literalPaths.length) {
      unresolved.push('nonliteral Rust path attribute: ' + source);
    }
  }
  return { inputs: [...inputs].sort(), unresolved: [...new Set(unresolved)].sort() };
}

// Audit specs name consumers and the runnable tests they are known to feed.
// They do NOT declare input file edges: those are extracted from the source.
export function auditSourceSelection({ specs, readSource, exists, planForPath }) {
  if (!Array.isArray(specs) || !specs.length ||
      typeof readSource !== 'function' || typeof exists !== 'function' ||
      typeof planForPath !== 'function') {
    throw new Error('incomplete source dependency audit');
  }
  const errors = [], edges = [], seen = new Set();
  for (const spec of specs) {
    if (!spec || Object.keys(spec).sort().join(',') !== 'requiredChecks,source' ||
        typeof spec.source !== 'string' || !spec.source || 
        path.isAbsolute(spec.source) || spec.source.includes('\\') ||
        path.normalize(spec.source).startsWith('../') ||
        path.normalize(spec.source) === '..' || seen.has(spec.source) ||
        !Array.isArray(spec.requiredChecks) || !spec.requiredChecks.length ||
        spec.requiredChecks.some(id => typeof id !== 'string' ||
          !/^(?:check:|cargo-test:)[a-z0-9-]+$/.test(id)) ||
        new Set(spec.requiredChecks).size !== spec.requiredChecks.length) {
      throw new Error('invalid source audit specification: ' + spec?.source);
    }
    seen.add(spec.source);
    let discovered;
    try {
      if (!exists(spec.source)) throw new Error('consumer does not exist');
      discovered = discoverLiteralSourceInputs(spec.source, readSource(spec.source));
      if (!discovered.inputs.length) errors.push(spec.source + ': no local inputs discovered');
    } catch (error) {
      errors.push(spec.source + ': ' + error.message);
      continue;
    }
    errors.push(...discovered.unresolved);
    for (const dependency of discovered.inputs) {
      if (!exists(dependency)) {
        errors.push(spec.source + ': missing local input ' + dependency);
        continue;
      }
      const plan = planForPath(dependency);
      if (!plan || typeof plan.full !== 'boolean' || !Array.isArray(plan.tests)) {
        errors.push('invalid impact plan for ' + dependency);
        continue;
      }
      const missing = plan.full ? [] : spec.requiredChecks.filter(id =>
        !plan.tests.includes(id));
      edges.push({ consumer: spec.source, dependency,
        outcome: plan.full ? 'full-fallback' : missing.length ? 'under-selected' : 'selected' });
      if (missing.length) errors.push(spec.source + ' -> ' + dependency +
        ': missing ' + missing.join(', '));
    }
  }
  return { valid: errors.length === 0, consumers: seen.size,
    edges, errors: [...new Set(errors)].sort() };
}
