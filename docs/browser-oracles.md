# Browser oracle track (independent of compiler/lowering)

This track measures **observable browser behavior**, not browser internals. It is
independent of the JS/TS compiler and the runtime's private IR. The initial
harness is intentionally dependency-free and does not claim Grunkalunka browser
compatibility.

## Two separate adapters

1. **Application host adapter** (future): run the same application scenario
   against a reference browser host and the Grunkalunka host, observing input,
   events, capability decisions, lifecycle and externally visible effects.
2. **Rendering oracle adapter** (this first slice): load an identical fixture
   into each reference engine and record selected DOM, CSS and geometry
   observations. The native renderer will eventually emit the same observation
   format. A reference engine is not a substitute implementation of the
   Grunkalunka DOM or layout architecture.

Blink (Chromium), Gecko (Firefox), and WebKit (Safari) are the independent
engine families. Chromium and Blink are not separate votes. Engine/version,
platform, viewport, device scale, fonts, zoom and capture mechanism must be
recorded. Never interpret engine agreement as a specification proof.

## First executable slice

Open `tests/browser-oracles/layout-baseline.html` in each browser and copy
the JSON displayed in the page. The fixture measures basic block geometry,
computed styles and text content. It uses only browser-native APIs and
requires no npm dependencies, server, or compiler changes.

The page deliberately **does not** report pass/fail. Observations are evidence,
not normative expectations. A later automated adapter should capture the same
payload using browser automation, pin browser versions, and store outputs as
disposable CI artifacts rather than checking machine-specific pixels into git.

## Observation contract, version 1

The JSON object contains:

- `schemaVersion: 1`
- `fixtureId` (stable string)
- `environment`: user agent, viewport CSS pixel width/height, device pixel
  ratio, document URL, and timestamp
- `observations`: ordered records with `id`, `text`, `computed` (selected
  computed style properties) and `rect` (CSS pixel x/y/width/height)

The fixture itself supplies the DOM and CSS; the observer must not normalize
layout or emulate browser behavior. Rects are floating-point CSS pixel values.
A comparator should compare fields individually, use explicit per-property
tolerances, distinguish missing/unsupported fields from zero, and retain raw
observations. Do not normalize away meaningful disagreements.

## Next slices

- Add a Node validator/comparator with structural errors, per-property
  tolerances and machine-readable disagreement reports.
- Add automated Blink/Gecko/WebKit collectors in a separate optional dependency
  boundary. Record browser versions and ensure all engines execute the same
  source fixture.
- Add WPT-derived cases with upstream provenance, licensing and specification
  links. Separate expected spec results, implementation-defined behavior, and
  cross-engine disagreements.
- Extend to DOM mutation, event ordering, CSS cascade, layout, paint and
  screenshot comparison. Screenshots require controlled fonts, scale, color
  management and platform metadata.
- Add a Grunkalunka observation adapter once a native DOM/layout slice exists.

No tests here imply that the browser engine, app host, or UI renderer exists.
Keep the browser oracle harness out of the compiler/lowering implementation
and preserve its ability to run against an independent browser.
