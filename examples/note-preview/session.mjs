// Private A1 host session: retain one authorized document and committed view
// between pointer, scroll, reload and note-selection inputs. Each source-service
// activation uses a fresh component Store; this does not retain guest globals or
// establish a public UI ABI.
import { spawnSync } from 'node:child_process';
import { prepareSelectedNoteScene } from './scene.mjs';
import { inspectNoteViewport, invokeUiService, paintNoteScene } from './service-interaction.mjs';

const COMPONENT_HEADER = Buffer.from([0, 97, 115, 109, 13, 0, 1, 0]);
const MAX_COMPONENT = 4 * 1024 * 1024;
const MAX_EVENTS = 256;
const MAX_NOTE_CHOICES = 8;
const MAX_NOTE_LABEL_BYTES = 256;

function ownComponent(component) {
  if (!(component instanceof Uint8Array) ||
      component.byteLength < COMPONENT_HEADER.length || component.byteLength > MAX_COMPONENT ||
      !Buffer.from(component.subarray(0, COMPONENT_HEADER.length)).equals(COMPONENT_HEADER)) {
    throw new TypeError('Expected bounded A1 component bytes');
  }
  return Buffer.from(component);
}

function ownNoteSelection(noteSelection) {
  if (noteSelection === undefined) return undefined;
  if (noteSelection === null || typeof noteSelection !== 'object' || Array.isArray(noteSelection)) {
    throw new TypeError('Invalid A1 note selection');
  }
  const { labels, selectedId, load } = noteSelection;
  const count = Array.isArray(labels) ? labels.length : 0;
  if (!Number.isSafeInteger(count) || count < 1 || count > MAX_NOTE_CHOICES ||
      !Number.isSafeInteger(selectedId) || selectedId < 0 || selectedId >= count ||
      typeof load !== 'function') {
    throw new TypeError('Invalid A1 note selection');
  }
  const copiedLabels = [];
  for (let id = 0; id < count; id++) {
    const label = labels[id];
    if (typeof label !== 'string' || label.length === 0 ||
        Buffer.byteLength(label, 'utf8') > MAX_NOTE_LABEL_BYTES || /[\u0000-\u001f\u007f]/.test(label)) {
      throw new TypeError('Invalid A1 note selection label');
    }
    copiedLabels.push(label);
  }
  return { labels: copiedLabels, selectedId, load };
}

function ownDocument(document) {
  if (document === null || typeof document !== 'object' ||
      !(document.sourceBytes instanceof Uint8Array) || document.sourceBytes.byteLength > 4096 ||
      !Array.isArray(document.headings) || document.headings.length > 64) {
    throw new TypeError('Expected a bounded authorized A1 document');
  }
  const owned = {
    profile: document.profile,
    backingReads: document.backingReads,
    sourceBytes: Buffer.from(document.sourceBytes),
    headings: document.headings.map(heading => {
      if (heading === null || typeof heading !== 'object' ||
          typeof heading.text !== 'string' || Buffer.byteLength(heading.text, 'utf8') > 4096) {
        throw new TypeError('Expected a bounded owned WIT heading');
      }
      return { level: heading.level, start: heading.start, end: heading.end,
        sectionEnd: heading.sectionEnd, parent: heading.parent, text: heading.text };
    }),
  };
  if (owned.profile === 'denied' && owned.sourceBytes.length !== 0) {
    throw new Error('Denied session disclosed protected bytes');
  }
  // Validate the complete owned projection even when the initial view is
  // collapsed. No caller-owned buffers or heading records survive this call.
  prepareSelectedNoteScene({ ...owned, visibleCount: 0 });
  return owned;
}

function sessionScene(document, expanded) {
  const scene = prepareSelectedNoteScene({ ...document,
    visibleCount: expanded ? document.headings.length : 0 });
  const previousWire = scene.input.toString('utf8');
  const header = ['A1SCENE2', document.profile, document.sourceBytes.byteLength,
    scene.visibleCount, document.backingReads, document.headings.length, expanded].join('\t');
  // Reuse the validated heading serialization and opt into the reversible
  // control protocol without changing the legacy static/one-shot scene.
  scene.input = Buffer.from(header + previousWire.slice(previousWire.indexOf('\n')), 'utf8');
  if (document.profile === 'granted' && document.headings.length === 0) {
    scene.html = scene.html.replace('<p id="outline-collapsed">Outline collapsed.</p>',
      '<p id="outline-empty">No headings.</p>');
  }
  return scene;
}

const copyScene = scene => ({ ...scene, input: Buffer.from(scene.input) });

function hitControl(executable, scene, x, y, scrollY) {
  const result = spawnSync(executable, ['--hit', String(x), String(y),
    ...(scrollY === 0 ? [] : ['--scroll-y', String(scrollY)])], {
    input: scene.input, shell: false, encoding: 'utf8', timeout: 60_000,
    killSignal: 'SIGKILL', maxBuffer: 4096,
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error('A1 native hit test failed: ' +
      (result.stderr?.trim() || result.error?.message || result.signal || result.status));
  }
  if (result.stdout === 'A1HIT2\tnone\n') return null;
  if (result.stdout === 'A1HIT2\ttoggle\n') return 'toggle';
  throw new Error('Malformed native A1 session hit response');
}

export function createNoteSession({
  executable, serviceExecutable, component, document, expanded = 1,
  uiProfile = 'granted', maxEvents = MAX_EVENTS, reloadDocument, noteSelection,
}) {
  if (![executable, serviceExecutable].every(value =>
    typeof value === 'string' && value.length > 0 && !value.includes('\0')) ||
      ![0, 1].includes(expanded) || !['granted', 'denied'].includes(uiProfile) ||
      !Number.isSafeInteger(maxEvents) || maxEvents < 1 || maxEvents > MAX_EVENTS ||
      (reloadDocument !== undefined && typeof reloadDocument !== 'function') ||
      (reloadDocument !== undefined && noteSelection !== undefined)) {
    throw new TypeError('Invalid A1 session arguments');
  }
  const selection = ownNoteSelection(noteSelection);
  const loadSelectedNote = selection?.load;
  let selectedNoteId = selection?.selectedId ?? null;
  // At most eight private reading positions, keyed by launch-catalog identity.
  // Only successful switches save the departing view. Unvisited notes use the
  // original launch preference, not the state of an unrelated active note.
  const savedViews = selection === undefined ? [] : Array(selection.labels.length);
  const payload = ownComponent(component);
  let owned = ownDocument(document);
  const documentProfile = owned.profile;
  let preferredExpanded = expanded;
  let currentExpanded = owned.profile === 'granted' && owned.headings.length > 0 ? preferredExpanded : 0;
  let currentScene = sessionScene(owned, currentExpanded);
  let currentViewport = inspectNoteViewport(executable, currentScene);
  let currentPixels = paintNoteScene(executable, currentScene);
  let eventsProcessed = 0;

  function snapshot() {
    return { expanded: currentExpanded, visibleCount: currentScene.visibleCount,
      scene: copyScene(currentScene), pixels: Buffer.from(currentPixels), eventsProcessed,
      scrollY: currentViewport.scrollY, maxScrollY: currentViewport.maxScrollY,
      controlPoint: currentViewport.controlPoint === null ? null : { ...currentViewport.controlPoint },
      noteChoices: documentProfile === 'granted' && selection !== undefined ?
        selection.labels.map((label, id) => ({ id, label })) : [],
      selectedNoteId: documentProfile === 'granted' ? selectedNoteId : null };
  }

  function nextAttempt() {
    if (eventsProcessed >= maxEvents) throw new RangeError('A1 session event limit exceeded');
    return ++eventsProcessed;
  }

  function scrollTo(y) {
    if (!Number.isSafeInteger(y) || y < 0 || y > 0xffffffff) {
      throw new TypeError('Invalid A1 session scroll offset');
    }
    const sequence = nextAttempt();
    const beforeScrollY = currentViewport.scrollY;
    const status = owned.profile === 'granted' && uiProfile === 'granted' ? 'ok' : 'denied';
    const nextY = Math.min(y, currentViewport.maxScrollY);
    if (status === 'ok' && nextY !== beforeScrollY) {
      const nextViewport = inspectNoteViewport(executable, currentScene, nextY);
      const nextPixels = paintNoteScene(executable, currentScene, nextY);
      // Viewport metadata and pixels form one publication. Failed inspection or
      // paint consumes the attempt while retaining the complete preceding view.
      currentViewport = nextViewport;
      currentPixels = nextPixels;
    }
    return { sequence, status, changed: currentViewport.scrollY !== beforeScrollY,
      beforeScrollY, scrollY: currentViewport.scrollY, pixels: Buffer.from(currentPixels) };
  }

  function replaceDocument(nextDocument, nextNoteId, scrollY, nextPreference = preferredExpanded) {
    const nextOwned = ownDocument(nextDocument);
    if (nextOwned.profile !== documentProfile) {
      throw new Error('Replacement A1 document profile differs from the original grant');
    }
    const nextExpanded = nextOwned.headings.length > 0 ? nextPreference : 0;
    const nextScene = sessionScene(nextOwned, nextExpanded);
    const topViewport = inspectNoteViewport(executable, nextScene);
    const nextY = Math.min(scrollY, topViewport.maxScrollY);
    const nextViewport = nextY === 0 ? topViewport :
      inspectNoteViewport(executable, nextScene, nextY);
    const nextPixels = paintNoteScene(executable, nextScene, nextY);
    const changed = !owned.sourceBytes.equals(nextOwned.sourceBytes) ||
      JSON.stringify(owned.headings) !== JSON.stringify(nextOwned.headings);
    // The reload target belongs to the same publication as the document and
    // native view. A failed selection cannot redirect a subsequent reload.
    owned = nextOwned;
    selectedNoteId = nextNoteId;
    preferredExpanded = nextPreference;
    currentExpanded = nextExpanded;
    currentScene = nextScene;
    currentViewport = nextViewport;
    currentPixels = nextPixels;
    return changed;
  }

  function reload() {
    if (reloadDocument === undefined && selection === undefined) {
      const error = new Error('A1 session reload is unavailable');
      error.code = 'A1_RELOAD_UNAVAILABLE';
      throw error;
    }
    const sequence = nextAttempt();
    if (documentProfile !== 'granted' || uiProfile !== 'granted') {
      return { sequence, status: 'denied', changed: false, pixels: Buffer.from(currentPixels) };
    }
    // Fixed trusted loaders run only after authority and lifetime admission.
    const noteId = selectedNoteId;
    const scrollY = currentViewport.scrollY;
    const nextDocument = loadSelectedNote === undefined ? reloadDocument() : loadSelectedNote(noteId);
    const changed = replaceDocument(nextDocument, noteId, scrollY);
    return { sequence, status: 'ok', changed, pixels: Buffer.from(currentPixels) };
  }

  function selectNote(id) {
    if (selection === undefined) {
      const error = new Error('A1 note selection is unavailable');
      error.code = 'A1_SELECTION_UNAVAILABLE';
      throw error;
    }
    if (!Number.isSafeInteger(id) || id < 0 || id >= selection.labels.length) {
      throw new TypeError('Invalid A1 note ID');
    }
    const sequence = nextAttempt();
    if (documentProfile !== 'granted' || uiProfile !== 'granted') {
      return { sequence, status: 'denied', changed: false, pixels: Buffer.from(currentPixels) };
    }
    if (id === selectedNoteId) {
      return { sequence, status: 'ok', changed: false, pixels: Buffer.from(currentPixels) };
    }
    const previousId = selectedNoteId;
    const previousView = { expanded: preferredExpanded, scrollY: currentViewport.scrollY };
    const nextView = savedViews[id] ?? { expanded, scrollY: 0 };
    replaceDocument(loadSelectedNote(id), id, nextView.scrollY, nextView.expanded);
    // Loading, validation, native inspection and paint must all succeed before
    // any saved state changes. A failed candidate also leaves its own prior
    // reading position intact for a later successful attempt.
    savedViews[previousId] = previousView;
    return { sequence, status: 'ok', changed: true, pixels: Buffer.from(currentPixels) };
  }

  function activate({ x, y }) {
    if (!Number.isSafeInteger(x) || !Number.isSafeInteger(y) ||
        x < 0 || y < 0 || x > 0xffffffff || y > 0xffffffff) {
      throw new TypeError('Invalid A1 session pointer');
    }
    // Count a valid attempt before native execution. Misses, policy failures and
    // traps cannot evade the session budget by leaving the view unchanged.
    const sequence = nextAttempt();
    const beforeExpanded = currentExpanded;
    const before = currentScene;
    const previousPixels = currentPixels;
    const target = hitControl(executable, before, x, y, currentViewport.scrollY);
    let service = null;
    let activated = false;
    if (target !== null) {
      const count = owned.headings.length;
      if (owned.profile !== 'granted' || count === 0) {
        throw new Error('Native A1 toggle disagrees with document authority');
      }
      service = invokeUiService({ executable: serviceExecutable, component: payload,
        profile: uiProfile, count, expanded: beforeExpanded });
      if (service.status === 'ok' && service.transitions > 0) {
        const nextExpanded = service.visible === count ? 1 : 0;
        const nextScene = sessionScene(owned, nextExpanded);
        const topViewport = inspectNoteViewport(executable, nextScene);
        const nextY = Math.min(currentViewport.scrollY, topViewport.maxScrollY);
        const nextViewport = nextY === 0 ? topViewport :
          inspectNoteViewport(executable, nextScene, nextY);
        const nextPixels = paintNoteScene(executable, nextScene, nextY);
        // Commit only after the native frame succeeds. A service failure, trap
        // or failed paint leaves the preceding successful scene intact.
        currentExpanded = nextExpanded;
        preferredExpanded = nextExpanded;
        currentScene = nextScene;
        currentViewport = nextViewport;
        currentPixels = nextPixels;
        activated = true;
      }
    }
    return { sequence, target, activated, changed: currentExpanded !== beforeExpanded,
      beforeExpanded, expanded: currentExpanded, before: copyScene(before),
      after: copyScene(currentScene), previousPixels: Buffer.from(previousPixels),
      pixels: Buffer.from(currentPixels), service };
  }

  return Object.freeze({ snapshot, activate, scrollTo, reload, selectNote });
}
