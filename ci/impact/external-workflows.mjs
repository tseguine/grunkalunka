// Advisory visibility into independent, path-filtered browser workflows.
// This does not infer that an expected workflow actually ran or passed.
function knownPath(path) {
  return typeof path === 'string' && path.length > 0 &&
    !path.startsWith('/') && !path.includes('\\') &&
    !path.split('/').includes('..') && !path.startsWith('!');
}

export function matchesOwnedPath(pattern, path) {
  if (!knownPath(pattern) || !knownPath(path)) throw new Error('invalid workflow path pattern');
  if (pattern.endsWith('/**')) return path.startsWith(pattern.slice(0, -2));
  if (pattern.includes('*') || pattern.includes('?') || pattern.includes('[')) {
    throw new Error('unsupported workflow glob: ' + pattern);
  }
  return path === pattern;
}

function parseList(value) {
  if (!value.startsWith('[') || !value.endsWith(']')) throw new Error('unsupported YAML inline list');
  const inside = value.slice(1, -1);
  if (!inside) return [];
  return inside.split(',').map(item => {
    const match = item.trim().match(/^'([^']+)'$/);
    if (!match || !knownPath(match[1])) throw new Error('unsupported YAML path: ' + item);
    return match[1];
  });
}

// Supports the two literal, single-quoted path-list layouts used by the
// checked-in browser workflows. Any new YAML feature is a drift/fallback,
// not permission to assume a trigger still covers the changed source.
export function parseBrowserTriggers(source) {
  if (typeof source !== 'string' || source.includes('\r') ||
      !source.startsWith('name:') && !source.startsWith('#')) {
    throw new Error('invalid workflow source');
  }
  const lines = source.split('\n');
  const onIndex = lines.indexOf('on:');
  if (onIndex < 0 || lines.lastIndexOf('on:') !== onIndex) {
    throw new Error('missing or duplicate on block');
  }
  const end = lines.findIndex((line, index) => index > onIndex &&
    /^[^\s#][^:]*:/.test(line));
  const block = lines.slice(onIndex + 1, end < 0 ? lines.length : end);
  if (!block.some(line => line === '  workflow_dispatch:')) {
    throw new Error('missing manual workflow trigger');
  }
  const output = {};
  for (const event of ['pull_request', 'push']) {
    const start = block.indexOf('  ' + event + ':');
    if (start < 0 || block.lastIndexOf('  ' + event + ':') !== start) {
      throw new Error('missing/duplicate ' + event + ' trigger');
    }
    const next = block.findIndex((line, i) => i > start && /^  \w[^:]*:/.test(line));
    const section = block.slice(start + 1, next < 0 ? block.length : next);
    if (!section.includes('    branches: [master]')) {
      throw new Error('unrecognized ' + event + ' branch filter');
    }
    if (section.some(line => /paths-ignore:|branches-ignore:/.test(line))) {
      throw new Error('unsupported negative workflow filter');
    }
    const pathLines = section.map((line, i) => ({ line, i }))
      .filter(x => /^    paths:/.test(x.line));
    if (pathLines.length !== 1) throw new Error('missing/duplicate ' + event + ' paths filter');
    const value = pathLines[0].line.slice('    paths:'.length).trim();
    let paths;
    if (value) {
      paths = parseList(value);
    } else {
      const remaining = section.slice(pathLines[0].i + 1);
      const candidates = remaining.slice(0,
        remaining.findIndex(line => /^    [^\s#]/.test(line)) < 0 ?
          remaining.length : remaining.findIndex(line => /^    [^\s#]/.test(line)));
      paths = candidates.filter(line => line.trim()).map(line => {
        const match = line.match(/^      - '([^']+)'$/);
        if (!match || !knownPath(match[1])) throw new Error('unrecognized path list item: ' + line);
        return match[1];
      });
    }
    if (!paths.length || new Set(paths).size !== paths.length) {
      throw new Error('empty or duplicate ' + event + ' path filter');
    }
    output[event] = paths;
  }
  return output;
}

export function auditBrowserWorkflows(changedPaths, descriptors, readWorkflow) {
  if (!Array.isArray(changedPaths) || !Array.isArray(descriptors) ||
      typeof readWorkflow !== 'function') {
    throw new Error('invalid external workflow audit inputs');
  }
  const errors = [], requiredNotTriggered = [], workflows = [], seen = new Set();
  for (const entry of descriptors) {
    const id = entry?.id, file = entry?.file;
    try {
      if (typeof id !== 'string' || !/^workflow:[a-z0-9-]+$/.test(id) ||
          seen.has(id) || !/^\.github\/workflows\/[a-z0-9-]+\.yml$/.test(file) ||
          !Array.isArray(entry.paths) || !Array.isArray(entry.requiredFor) ||
          !Array.isArray(entry.mustContain) || !entry.paths.length ||
          !entry.requiredFor.length || !entry.mustContain.length) {
        throw new Error('invalid workflow descriptor');
      }
      seen.add(id);
      const source = readWorkflow(file);
      const triggers = parseBrowserTriggers(source);
      for (const event of ['pull_request', 'push']) {
        if (JSON.stringify(triggers[event]) !== JSON.stringify(entry.paths)) {
          throw new Error(event + ' filter differs from descriptor');
        }
      }
      for (const text of entry.mustContain) {
        if (typeof text !== 'string' || !text.length || !source.includes(text)) {
          throw new Error('missing required executable evidence: ' + text);
        }
      }
      const requiredPaths = changedPaths.filter(path =>
        entry.requiredFor.some(pattern => matchesOwnedPath(pattern, path)));
      const triggeredPaths = changedPaths.filter(path =>
        triggers.pull_request.some(pattern => matchesOwnedPath(pattern, path)));
      const triggered = triggeredPaths.length > 0;
      if (requiredPaths.length && !triggered) requiredNotTriggered.push(id);
      workflows.push({
        id, file, required: requiredPaths.length > 0, triggered,
        triggeredBy: triggeredPaths, requiredBy: requiredPaths,
        execution: triggered ? 'expected-by-path-filter; result not inspected' : 'not-triggered',
      });
    } catch (error) {
      errors.push(String(id ?? file ?? 'unknown') + ': ' + error.message);
    }
  }
  return {
    advisoryOnly: true, scope: 'independent browser-oracle workflows only',
    valid: errors.length === 0,
    expectedToTrigger: workflows.filter(w => w.triggered).map(w => w.id),
    requiredNotTriggered, errors, workflows,
  };
}
