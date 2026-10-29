// Selected native block-border geometry versus a real browser receipt.
// This is not the generic browser-vs-browser comparator or a CSS conformance test.
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { validateCapture } from "./compare.mjs";

const ids = ["outer", "first", "second"];
const heightIds = ["height-host", "height-first", "height-second"];
const borderIds = ["border-host", "border-first", "border-second"];
const widthBoundIds = ["bounds-host", "bounds-first", "bounds-second"];
const heightBoundIds = ["height-limited", "height-limited-first", "height-limited-second"];
const visibilityIds = ["visibility-host", "visibility-child",
  "visibility-inherited", "visibility-sibling"];
const visibilityProbes = ["child", "host", "inherited", "sibling"];
const visibilityPhases = ["hidden", "visible", "restored"];
const clipPhases = ["clipped", "visible", "restored"];
const clipIds = ["clip-parent", "clip-child", "clip-sibling"];
const clipProbes = ["inside", "padding-edge", "border-edge", "outside-parent",
  "bottom-border", "sibling", "parent-border"];
const boxlessPhases = ["contents", "block", "restored"];
const boxlessIds = ["contents-first", "contents-second", "contents-sibling"];
const boxlessTargets = ["contents-wrapper", ...boxlessIds];
const boxlessProbes = ["first-contents", "second-contents", "wrapper-border",
  "first-block", "sibling-contents", "sibling-block", "outside"];
const pointerPhases = ["none", "auto", "restored"];
const pointerIds = ["pointer-host", "pointer-inherited",
  "pointer-restored", "pointer-sibling"];
const pointerProbes = ["inherited", "host", "restored",
  "host-lower", "sibling", "outside"];
const autoMarginPhases = ["centered", "leftFixed", "restored"];
const autoMarginIds = ["auto-host", "auto-centered", "auto-left", "auto-right",
  "auto-limited", "auto-sibling"];
const autoMarginChildren = ["auto-centered", "auto-left", "auto-right", "auto-limited"];
const autoMarginProbes = ["center-original", "center-shifted", "left-auto",
  "right-auto", "limited", "host", "sibling", "outside"];
const relativePhases = ["baseline", "parentMoved", "restored"];
const relativeIds = ["relative-host", "relative-child", "relative-sibling"];
const relativeStyled = ["relative-host", "relative-child"];
const relativeProbes = ["original-parent", "moved-parent", "original-child",
  "moved-child", "host-interior", "sibling", "outside"];
const percentPhases = ["original", "changed", "restored"];
const percentIds = ["percent-host", "percent-first", "percent-next", "percent-after"];
const percentSides = ["top", "right", "bottom", "left"];
const percentProbes = ["baseline-only", "shifted-only", "first-center",
  "top-margin", "next-in-flow", "host-bottom", "following-sibling", "outside"];
const scrollPhases = ["before", "scrolled", "restored"];
const probes = ["first-center", "first-lower", "second-original", "left-margin",
  "outer-border", "outside", "first-right"];
const rectFields = ["x", "y", "width", "height"];
const computedFields = ["display", "boxSizing", "width", "height", "marginTop",
  "paddingLeft", "borderLeftWidth"];
const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const record = value => value !== null && typeof value === "object" && !Array.isArray(value);
const hash = bytes => createHash("sha256").update(bytes).digest("hex");

export function compareNativeBlockBridge(native, browser, sourceBytes) {
  if (!(sourceBytes instanceof Uint8Array)) throw new TypeError("fixture bytes required");
  const sourceSha256 = hash(sourceBytes);
  const invalid = [];
  const incompatible = [];
  const differences = [];
  const error = (path, message) => invalid.push({ path, message });
  const mismatch = (path, left, right) => incompatible.push({ path, left, right });
  const report = {
    schemaVersion: 1, kind: "grunkalunka-native-browser-geometry",
    status: "invalid", conformance: "not-assessed", sourceSha256,
    comparisonScope: "selected block, dimensions, scroll, visibility, padding clips, boxless contents, pointer-events, auto margins and relative offsets and percentage-margin geometry/hits; no HTML/CSS parsing, inline layout or paint",
    invalid, incompatible, differences, native, browser,
  };
  if (!record(native) || native.schemaVersion !== 1 ||
      native.kind !== "grunkalunka-native-block-geometry" ||
      native.fixtureId !== "native-block-bridge-v1") {
    error("$.native", "expected a native block bridge v1 receipt");
  } else {
    if (!record(native.viewport) ||
        native.viewport.width !== 800 || native.viewport.height !== 600 ||
        native.viewport.devicePixelRatio !== 1) {
      error("$.native.viewport", "expected the fixed 800x600 DPR1 bridge viewport");
    }
    if (!record(native.geometry) ||
        Object.keys(native.geometry).sort().join(",") !== [...ids].sort().join(",")) {
      error("$.native.geometry", "exactly the three named native rectangles required");
    } else {
      for (const id of ids) {
        const rect = native.geometry[id];
        for (const field of rectFields) {
          if (!record(rect) || !Number.isFinite(rect[field]) ||
              (["width", "height"].includes(field) && rect[field] < 0)) {
            error("$.native.geometry." + id + "." + field, "finite rectangle coordinate required");
          }
        }
      }
    }
  }
  for (const item of validateCapture(browser)) error("$.browser" + item.path.slice(1), item.message);

  // The collector preserves extra fixture observations without rewriting
  // them. Both the native and browser records must independently include all
  // reflow states: missing data must never pass as an unchanged rectangle.
  const phases = ["hidden", "restored"];
  const sortedIds = [...ids].sort().join(",");
  function validateReflow(name, value) {
    if (!record(value) || Object.keys(value).sort().join(",") !== phases.join(",")) {
      error("$." + name + ".reflow", "exactly hidden and restored phases required");
      return;
    }
    for (const phase of phases) {
      const state = value[phase];
      if (!record(state) || Object.keys(state).sort().join(",") !== sortedIds) {
        error("$." + name + ".reflow." + phase, "three named rectangles required");
        continue;
      }
      for (const id of ids) {
        const box = state[id];
        if (box === null) continue; // Explicit display:none; never missing/undefined.
        if (!record(box) || Object.keys(box).sort().join(",") !== [...rectFields].sort().join(",")) {
          error("$." + name + ".reflow." + phase + "." + id, "complete rectangle or null required");
          continue;
        }
        for (const field of rectFields) {
          if (!Number.isFinite(box[field]) ||
              ((field === "width" || field === "height") && box[field] < 0)) {
            error("$." + name + ".reflow." + phase + "." + id + "." + field,
              "finite CSS-pixel rectangle coordinate required");
          }
        }
      }
    }
  }
  validateReflow("native", native?.reflow);
  validateReflow("browser", browser?.reflow);

  // Independent CSS percentage-height observations remain a separate
  // measurement channel; incomplete boxes never count as agreement.
  function validateHeightBoxes(owner, value) {
    const path = "$." + owner + ".percentageHeights";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...heightIds].sort().join(",")) {
      error(path, "exactly the three named percentage-height border boxes required");
      return;
    }
    for (const id of heightIds) {
      const box = value[id];
      if (!record(box) || Object.keys(box).sort().join(",") !== [...rectFields].sort().join(",")) {
        error(path + "." + id, "complete percentage-height rectangle required");
        continue;
      }
      for (const field of rectFields) {
        if (!Number.isFinite(box[field]) ||
            (["width", "height"].includes(field) && box[field] < 0)) {
          error(path + "." + id + "." + field, "finite CSS-pixel rectangle required");
        }
      }
    }
  }
  validateHeightBoxes("native", native?.percentageHeights);
  validateHeightBoxes("browser", browser?.percentageHeights);

  function validateBorderBoxes(owner, value) {
    const path = "$." + owner + ".borderBoxes";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...borderIds].sort().join(",")) {
      error(path, "exactly the three named border-box rectangles required");
      return;
    }
    for (const id of borderIds) {
      const box = value[id];
      if (!record(box) || Object.keys(box).sort().join(",") !== [...rectFields].sort().join(",")) {
        error(path + "." + id, "complete border-box rectangle required");
        continue;
      }
      for (const field of rectFields) {
        if (!Number.isFinite(box[field]) ||
            (["width", "height"].includes(field) && box[field] < 0)) {
          error(path + "." + id + "." + field, "finite CSS-pixel rectangle required");
        }
      }
    }
  }
  validateBorderBoxes("native", native?.borderBoxes);
  validateBorderBoxes("browser", browser?.borderBoxes);

  // Source-captured min/max-width rectangles are an independent geometry
  // channel. A missing field is not a zero or an observed equal.
  function validateWidthBounds(owner, value) {
    const path = "$." + owner + ".widthBounds";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...widthBoundIds].sort().join(",")) {
      error(path, "exactly the three named constrained-width boxes required");
      return;
    }
    for (const id of widthBoundIds) {
      const box = value[id];
      if (!record(box) || Object.keys(box).sort().join(",") !== [...rectFields].sort().join(",")) {
        error(path + "." + id, "complete constrained-width rectangle required");
        continue;
      }
      for (const field of rectFields) {
        if (!Number.isFinite(box[field]) ||
            (["width", "height"].includes(field) && box[field] < 0)) {
          error(path + "." + id + "." + field, "finite CSS-pixel rectangle required");
        }
      }
    }
  }
  validateWidthBounds("native", native?.widthBounds);
  validateWidthBounds("browser", browser?.widthBounds);

  function validateHeightBounds(owner, value) {
    const path = "$." + owner + ".heightBounds";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...heightBoundIds].sort().join(",")) {
      error(path, "exactly three named constrained-height boxes required");
      return;
    }
    for (const id of heightBoundIds) {
      const box = value[id];
      if (!record(box) || Object.keys(box).sort().join(",") !== [...rectFields].sort().join(",")) {
        error(path + "." + id, "complete constrained-height rectangle required");
        continue;
      }
      for (const field of rectFields) {
        if (!Number.isFinite(box[field]) ||
            (["width", "height"].includes(field) && box[field] < 0)) {
          error(path + "." + id + "." + field, "finite CSS-pixel rectangle required");
        }
      }
    }
  }
  validateHeightBounds("native", native?.heightBounds);
  validateHeightBounds("browser", browser?.heightBounds);

  function validateVisibilityStates(owner, states) {
    const path = "$." + owner + ".visibilityStates";
    if (!record(states) ||
        Object.keys(states).sort().join(",") !== [...visibilityPhases].sort().join(",")) {
      error(path, "exactly hidden, visible and restored visibility phases required");
      return;
    }
    const knownIds = [...visibilityIds].sort().join(",");
    const knownProbes = [...visibilityProbes].sort().join(",");
    const knownRectFields = [...rectFields].sort().join(",");
    for (const phase of visibilityPhases) {
      const sample = states[phase];
      const prefix = path + "." + phase;
      if (!record(sample) ||
          Object.keys(sample).sort().join(",") !== "boxes,computed,hits") {
        error(prefix, "complete visibility boxes, computed state and hit probes required");
        continue;
      }
      if (!record(sample.boxes) || Object.keys(sample.boxes).sort().join(",") !== knownIds) {
        error(prefix + ".boxes", "exactly four visible/hidden geometry nodes required");
      } else {
        for (const id of visibilityIds) {
          const rect = sample.boxes[id];
          if (!record(rect) ||
              Object.keys(rect).sort().join(",") !== knownRectFields) {
            error(prefix + ".boxes." + id, "complete visibility rectangle required");
            continue;
          }
          for (const field of rectFields) {
            if (!Number.isFinite(rect[field]) ||
                (["width", "height"].includes(field) && rect[field] < 0)) {
              error(prefix + ".boxes." + id + "." + field,
                "finite CSS-pixel rectangle required");
            }
          }
        }
      }
      if (!record(sample.computed) ||
          Object.keys(sample.computed).sort().join(",") !== knownIds) {
        error(prefix + ".computed", "exactly four computed visibility states required");
      } else {
        for (const id of visibilityIds) {
          if (typeof sample.computed[id] !== "boolean") {
            error(prefix + ".computed." + id, "resolved visibility Boolean required");
          }
        }
      }
      if (!record(sample.hits) ||
          Object.keys(sample.hits).sort().join(",") !== knownProbes) {
        error(prefix + ".hits", "exactly four DOM input-target probes required");
      } else {
        for (const probe of visibilityProbes) {
          const target = sample.hits[probe];
          if (target !== null && !visibilityIds.includes(target)) {
            error(prefix + ".hits." + probe, "known ID or explicit null required");
          }
        }
      }
    }
  }
  validateVisibilityStates("native", native?.visibilityStates);
  validateVisibilityStates("browser", browser?.visibilityStates);

  function validateDescendantClipStates(owner, value) {
    const path = "$." + owner + ".descendantClipStates";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...clipPhases].sort().join(",")) {
      error(path, "clipped, visible and restored descendant-clip phases required");
      return;
    }
    const expectedIds = [...clipIds].sort().join(",");
    const expectedProbes = [...clipProbes].sort().join(",");
    const expectedFields = [...rectFields].sort().join(",");
    for (const phase of clipPhases) {
      const sample = value[phase], prefix = path + "." + phase;
      if (!record(sample) ||
          Object.keys(sample).sort().join(",") !== "boxes,hits,mode") {
        error(prefix, "exact clip mode, boxes and hit probes required");
        continue;
      }
      if (!["clip", "visible"].includes(sample.mode)) {
        error(prefix + ".mode", "source-observed clip/visible mode required");
      }
      if (!record(sample.boxes) ||
          Object.keys(sample.boxes).sort().join(",") !== expectedIds) {
        error(prefix + ".boxes", "exactly the three known clip boxes required");
      } else {
        for (const id of clipIds) {
          const rect = sample.boxes[id];
          if (!record(rect) || Object.keys(rect).sort().join(",") !== expectedFields) {
            error(prefix + ".boxes." + id, "complete clip rectangle required");
            continue;
          }
          for (const field of rectFields) {
            if (!Number.isFinite(rect[field]) ||
                (["width", "height"].includes(field) && rect[field] < 0)) {
              error(prefix + ".boxes." + id + "." + field,
                "finite CSS-pixel rectangle required");
            }
          }
        }
      }
      if (!record(sample.hits) ||
          Object.keys(sample.hits).sort().join(",") !== expectedProbes) {
        error(prefix + ".hits", "exactly seven clip hit probes required");
      } else {
        for (const probe of clipProbes) {
          const target = sample.hits[probe];
          if (target !== null && !clipIds.includes(target)) {
            error(prefix + ".hits." + probe, "known clip target or explicit null required");
          }
        }
      }
    }
  }
  validateDescendantClipStates("native", native?.descendantClipStates);
  validateDescendantClipStates("browser", browser?.descendantClipStates);

  function validateBoxlessStates(owner, states) {
    const path = "$." + owner + ".boxlessStates";
    if (!record(states) ||
        Object.keys(states).sort().join(",") !== [...boxlessPhases].sort().join(",")) {
      error(path, "boxless contents, block and restored phases required");
      return;
    }
    const expectedBoxes = [...boxlessIds].sort().join(",");
    const expectedProbes = [...boxlessProbes].sort().join(",");
    const expectedFields = [...rectFields].sort().join(",");
    for (const phase of boxlessPhases) {
      const sample = states[phase], prefix = path + "." + phase;
      if (!record(sample) ||
          Object.keys(sample).sort().join(",") !== "boxes,display,hasBox,hits") {
        error(prefix, "complete boxless geometry, display, box-presence and hits required");
        continue;
      }
      if (!["contents", "block"].includes(sample.display)) {
        error(prefix + ".display", "source-observed display mode required");
      }
      if (typeof sample.hasBox !== "boolean") {
        error(prefix + ".hasBox", "actual principal-box presence required");
      }
      if (!record(sample.boxes) ||
          Object.keys(sample.boxes).sort().join(",") !== expectedBoxes) {
        error(prefix + ".boxes", "exactly the three boxless children/sibling required");
      } else {
        for (const id of boxlessIds) {
          const rect = sample.boxes[id];
          if (!record(rect) || Object.keys(rect).sort().join(",") !== expectedFields) {
            error(prefix + ".boxes." + id, "complete child/sibling rectangle required");
            continue;
          }
          for (const field of rectFields) {
            if (!Number.isFinite(rect[field]) ||
                (["width", "height"].includes(field) && rect[field] < 0)) {
              error(prefix + ".boxes." + id + "." + field,
                "finite CSS-pixel rectangle required");
            }
          }
        }
      }
      if (!record(sample.hits) ||
          Object.keys(sample.hits).sort().join(",") !== expectedProbes) {
        error(prefix + ".hits", "exactly seven independent boxless hit probes required");
      } else {
        for (const name of boxlessProbes) {
          const target = sample.hits[name];
          if (target !== null && !boxlessTargets.includes(target)) {
            error(prefix + ".hits." + name, "known boxless node or explicit null required");
          }
        }
      }
    }
  }
  validateBoxlessStates("native", native?.boxlessStates);
  validateBoxlessStates("browser", browser?.boxlessStates);

  function validatePointerStates(owner, states) {
    const path = "$." + owner + ".pointerStates";
    if (!record(states) ||
        Object.keys(states).sort().join(",") !== [...pointerPhases].sort().join(",")) {
      error(path, "none, auto and restored pointer-events phases required");
      return;
    }
    const exactIds = [...pointerIds].sort().join(",");
    const exactProbes = [...pointerProbes].sort().join(",");
    const exactFields = [...rectFields].sort().join(",");
    for (const phase of pointerPhases) {
      const sample = states[phase], prefix = path + "." + phase;
      if (!record(sample) ||
          Object.keys(sample).sort().join(",") !== "boxes,computed,hits") {
        error(prefix, "complete pointer geometry, computed styles and hits required");
        continue;
      }
      if (!record(sample.boxes) || Object.keys(sample.boxes).sort().join(",") !== exactIds) {
        error(prefix + ".boxes", "exactly four pointer witness boxes required");
      } else {
        for (const id of pointerIds) {
          const rect = sample.boxes[id];
          if (!record(rect) || Object.keys(rect).sort().join(",") !== exactFields) {
            error(prefix + ".boxes." + id, "complete pointer rectangle required");
            continue;
          }
          for (const field of rectFields) {
            if (!Number.isFinite(rect[field]) ||
                (["width", "height"].includes(field) && rect[field] < 0)) {
              error(prefix + ".boxes." + id + "." + field,
                "finite CSS-pixel rectangle required");
            }
          }
        }
      }
      if (!record(sample.computed) ||
          Object.keys(sample.computed).sort().join(",") !== exactIds) {
        error(prefix + ".computed", "exactly four inherited pointer values required");
      } else {
        for (const id of pointerIds) {
          if (typeof sample.computed[id] !== "boolean") {
            error(prefix + ".computed." + id, "resolved pointer eligibility Boolean required");
          }
        }
      }
      if (!record(sample.hits) ||
          Object.keys(sample.hits).sort().join(",") !== exactProbes) {
        error(prefix + ".hits", "exactly six pointer input probes required");
      } else {
        for (const probe of pointerProbes) {
          const id = sample.hits[probe];
          if (id !== null && !pointerIds.includes(id)) {
            error(prefix + ".hits." + probe, "known pointer element or explicit null required");
          }
        }
      }
    }
  }
  validatePointerStates("native", native?.pointerStates);
  validatePointerStates("browser", browser?.pointerStates);

  function validateAutoMarginStates(owner, states) {
    const path = "$." + owner + ".autoMarginStates";
    if (!record(states) ||
        Object.keys(states).sort().join(",") !== [...autoMarginPhases].sort().join(",")) {
      error(path, "centered, leftFixed and restored auto-margin phases required");
      return;
    }
    const exactIds = [...autoMarginIds].sort().join(",");
    const exactChildren = [...autoMarginChildren].sort().join(",");
    const exactProbes = [...autoMarginProbes].sort().join(",");
    const exactFields = [...rectFields].sort().join(",");
    for (const phase of autoMarginPhases) {
      const sample = states[phase], prefix = path + "." + phase;
      if (!record(sample) ||
          Object.keys(sample).sort().join(",") !== "boxes,computed,hits") {
        error(prefix, "complete auto-margin geometry, used margins and hits required");
        continue;
      }
      if (!record(sample.boxes) || Object.keys(sample.boxes).sort().join(",") !== exactIds) {
        error(prefix + ".boxes", "exactly six auto-margin boxes required");
      } else {
        for (const id of autoMarginIds) {
          const rect = sample.boxes[id];
          if (!record(rect) || Object.keys(rect).sort().join(",") !== exactFields) {
            error(prefix + ".boxes." + id, "complete auto-margin rectangle required");
            continue;
          }
          for (const field of rectFields) {
            if (!Number.isFinite(rect[field]) ||
                (["width", "height"].includes(field) && rect[field] < 0)) {
              error(prefix + ".boxes." + id + "." + field,
                "finite CSS-pixel rectangle required");
            }
          }
        }
      }
      if (!record(sample.computed) ||
          Object.keys(sample.computed).sort().join(",") !== exactChildren) {
        error(prefix + ".computed", "exactly four used horizontal margin pairs required");
      } else {
        for (const id of autoMarginChildren) {
          const used = sample.computed[id];
          if (!record(used) || Object.keys(used).sort().join(",") !== "left,right") {
            error(prefix + ".computed." + id, "both used horizontal margins required");
            continue;
          }
          for (const side of ["left", "right"]) {
            if (typeof used[side] !== "string" ||
                !/^\d+(?:\.\d+)?px$/.test(used[side])) {
              error(prefix + ".computed." + id + "." + side,
                "nonnegative browser-computed CSS pixel margin required");
            }
          }
        }
      }
      if (!record(sample.hits) ||
          Object.keys(sample.hits).sort().join(",") !== exactProbes) {
        error(prefix + ".hits", "exactly eight auto-margin input probes required");
      } else {
        for (const probe of autoMarginProbes) {
          const id = sample.hits[probe];
          if (id !== null && !autoMarginIds.includes(id)) {
            error(prefix + ".hits." + probe,
              "known auto-margin element or explicit null required");
          }
        }
      }
    }
  }
  validateAutoMarginStates("native", native?.autoMarginStates);
  validateAutoMarginStates("browser", browser?.autoMarginStates);

  function validateRelativeStates(owner, states) {
    const path = "$." + owner + ".relativeStates";
    if (!record(states) ||
        Object.keys(states).sort().join(",") !== [...relativePhases].sort().join(",")) {
      error(path, "baseline, parentMoved and restored relative phases required");
      return;
    }
    const exactIds = [...relativeIds].sort().join(",");
    const exactStyled = [...relativeStyled].sort().join(",");
    const exactProbes = [...relativeProbes].sort().join(",");
    const exactFields = [...rectFields].sort().join(",");
    for (const phase of relativePhases) {
      const sample = states[phase], prefix = path + "." + phase;
      if (!record(sample) ||
          Object.keys(sample).sort().join(",") !== "boxes,computed,hits") {
        error(prefix, "complete relative geometry, computed offsets and hits required");
        continue;
      }
      if (!record(sample.boxes) || Object.keys(sample.boxes).sort().join(",") !== exactIds) {
        error(prefix + ".boxes", "exactly three relative boxes required");
      } else {
        for (const id of relativeIds) {
          const box = sample.boxes[id];
          if (!record(box) || Object.keys(box).sort().join(",") !== exactFields) {
            error(prefix + ".boxes." + id, "complete relative border rectangle required");
            continue;
          }
          for (const field of rectFields) {
            if (!Number.isFinite(box[field]) ||
                (["width", "height"].includes(field) && box[field] < 0)) {
              error(prefix + ".boxes." + id + "." + field,
                "finite nonnegative-size CSS pixel rectangle required");
            }
          }
        }
      }
      if (!record(sample.computed) ||
          Object.keys(sample.computed).sort().join(",") !== exactStyled) {
        error(prefix + ".computed", "exactly two positioned computed styles required");
      } else {
        for (const id of relativeStyled) {
          const style = sample.computed[id];
          if (!record(style) ||
              Object.keys(style).sort().join(",") !== "left,position,top") {
            error(prefix + ".computed." + id, "position, left and top required");
            continue;
          }
          if (style.position !== "relative") {
            error(prefix + ".computed." + id + ".position", "source-observed relative required");
          }
          for (const side of ["left", "top"]) {
            if (typeof style[side] !== "string" ||
                !/^\d+(?:\.\d+)?px$/.test(style[side])) {
              error(prefix + ".computed." + id + "." + side,
                "nonnegative resolved CSS-pixel offset required");
            }
          }
        }
      }
      if (!record(sample.hits) ||
          Object.keys(sample.hits).sort().join(",") !== exactProbes) {
        error(prefix + ".hits", "exactly seven independent relative hit probes required");
      } else {
        for (const probe of relativeProbes) {
          const id = sample.hits[probe];
          if (id !== null && !relativeIds.includes(id)) {
            error(prefix + ".hits." + probe,
              "known relative element or explicit null required");
          }
        }
      }
    }
  }
  validateRelativeStates("native", native?.relativeStates);
  validateRelativeStates("browser", browser?.relativeStates);

  function validatePercentStates(owner, states) {
    const path = "$." + owner + ".percentStates";
    if (!record(states) ||
        Object.keys(states).sort().join(",") !== [...percentPhases].sort().join(",")) {
      error(path, "original, changed and restored percent-margin phases required");
      return;
    }
    const exactIds = [...percentIds].sort().join(",");
    const exactSides = [...percentSides].sort().join(",");
    const exactProbes = [...percentProbes].sort().join(",");
    const rectKeys = [...rectFields].sort().join(",");
    for (const phase of percentPhases) {
      const sample = states[phase], prefix = path + "." + phase;
      if (!record(sample) ||
          Object.keys(sample).sort().join(",") !== "boxes,computed,hits,scrollY") {
        error(prefix, "complete percentage geometry, used margins, scroll and hits required");
        continue;
      }
      if (sample.scrollY !== 200) {
        error(prefix + ".scrollY", "actual reference viewport scroll must be 200px");
      }
      if (!record(sample.boxes) || Object.keys(sample.boxes).sort().join(",") !== exactIds) {
        error(prefix + ".boxes", "exactly four percentage group rectangles required");
      } else {
        for (const id of percentIds) {
          const box = sample.boxes[id];
          if (!record(box) || Object.keys(box).sort().join(",") !== rectKeys) {
            error(prefix + ".boxes." + id, "complete document-space rectangle required");
            continue;
          }
          for (const field of rectFields) {
            if (!Number.isFinite(box[field]) ||
                (["width", "height"].includes(field) && box[field] < 0)) {
              error(prefix + ".boxes." + id + "." + field,
                "finite nonnegative-size CSS-pixel rectangle required");
            }
          }
        }
      }
      if (!record(sample.computed) ||
          Object.keys(sample.computed).sort().join(",") !== exactSides) {
        error(prefix + ".computed", "all four used percentage margin sides required");
      } else {
        for (const side of percentSides) {
          const value = sample.computed[side];
          if (typeof value !== "string" || !/^\d+(?:\.\d+)?px$/.test(value)) {
            error(prefix + ".computed." + side, "nonnegative CSS-pixel used margin required");
          }
        }
      }
      if (!record(sample.hits) ||
          Object.keys(sample.hits).sort().join(",") !== exactProbes) {
        error(prefix + ".hits", "exactly eight scrolled percentage hit probes required");
      } else {
        for (const probe of percentProbes) {
          const target = sample.hits[probe];
          if (target !== null && !percentIds.includes(target)) {
            error(prefix + ".hits." + probe,
              "known percent-margin element or explicit null required");
          }
        }
      }
    }
  }
  validatePercentStates("native", native?.percentStates);
  validatePercentStates("browser", browser?.percentStates);

  function validateScrollViewport(owner, value) {
    const path = "$." + owner + ".scrollViewport";
    const expected = [...scrollPhases, "documentHeight"].sort().join(",");
    if (!record(value) || Object.keys(value).sort().join(",") !== expected) {
      error(path, "document height and exactly three scroll phases required");
      return;
    }
    if (!Number.isSafeInteger(value.documentHeight) || value.documentHeight < 600) {
      error(path + ".documentHeight", "valid document extent required");
    }
    for (const phase of scrollPhases) {
      const sample = value[phase];
      const point = path + "." + phase;
      if (!record(sample) ||
          Object.keys(sample).sort().join(",") !==
            ["scrollX", "scrollY", "targetRect", "hitTarget"].sort().join(",")) {
        error(point, "complete document-scroll sample required");
        continue;
      }
      for (const key of ["scrollX", "scrollY"]) {
        if (!Number.isSafeInteger(sample[key]) || sample[key] < 0) {
          error(point + "." + key, "finite nonnegative scroll position required");
        }
      }
      if (sample.hitTarget !== null && sample.hitTarget !== "scroll-target") {
        error(point + ".hitTarget", "known scroll target or explicit null required");
      }
      const rect = sample.targetRect;
      if (!record(rect) || Object.keys(rect).sort().join(",") !== [...rectFields].sort().join(",")) {
        error(point + ".targetRect", "complete target rectangle required");
        continue;
      }
      for (const field of rectFields) {
        if (!Number.isFinite(rect[field]) ||
            (["width", "height"].includes(field) && rect[field] < 0)) {
          error(point + ".targetRect." + field, "finite CSS-pixel rectangle required");
        }
      }
    }
  }
  validateScrollViewport("native", native?.scrollViewport);
  validateScrollViewport("browser", browser?.scrollViewport);

  const targetPhases = ["original", "hidden", "restored"];
  function validateTargets(owner, value) {
    const path = "$." + owner + ".hitTargets";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...targetPhases].sort().join(",")) {
      error(path, "exactly original, hidden and restored target phases required");
      return;
    }
    for (const phase of targetPhases) {
      const map = value[phase];
      if (!record(map) || Object.keys(map).sort().join(",") !== [...probes].sort().join(",")) {
        error(path + "." + phase, "exactly the seven named hit probes required");
        continue;
      }
      for (const probe of probes) {
        if (map[probe] !== null && !ids.includes(map[probe])) {
          error(path + "." + phase + "." + probe, "known target ID or explicit null required");
        }
      }
    }
  }
  validateTargets("native", native?.hitTargets);
  validateTargets("browser", browser?.hitTargets);
  if (!invalid.length) {
    if (browser.fixtureId !== native.fixtureId) {
      mismatch("$.fixtureId", native.fixtureId, browser.fixtureId);
    }
    if (browser.environment.viewportWidth !== native.viewport.width ||
        browser.environment.viewportHeight !== native.viewport.height ||
        browser.environment.devicePixelRatio !== native.viewport.devicePixelRatio) {
      mismatch("$.environment.viewport", native.viewport, browser.environment);
    }
    if (browser.observations.length !== ids.length ||
        !ids.every((id, index) => browser.observations[index]?.id === id)) {
      mismatch("$.observations", ids, browser.observations.map(o => o.id));
    }
    const p = browser.provenance;
    if (!record(p) || p.collector !== "playwright" ||
        !Object.hasOwn(families, p.engine) || p.engineFamily !== families[p.engine] ||
        !["browserVersion", "playwrightVersion", "hostPlatform", "hostArchitecture", "nodeVersion"]
          .every(key => typeof p[key] === "string" && p[key].trim() && p[key] !== "unrecorded") ||
        !record(p.viewport) || p.viewport.width !== native.viewport.width ||
        p.viewport.height !== native.viewport.height ||
        p.viewport.devicePixelRatio !== native.viewport.devicePixelRatio ||
        p.fixtureSha256 !== sourceSha256) {
      mismatch("$.provenance", "matching Playwright source receipt, engine and viewport", p ?? null);
    }
    for (const [index, id] of ids.entries()) {
      const obs = browser.observations[index];
      for (const key of computedFields) {
        if (!Object.hasOwn(obs?.computed ?? {}, key) || !obs.computed[key]) {
          error("$.browser.observations[" + index + "].computed." + key,
            "required browser-observed computed property");
        }
      }
      if (obs?.text !== "") {
        mismatch("$.observations[" + index + "].text", "", obs?.text ?? null);
      }
    }
  }
  if (invalid.length) return report;
  if (incompatible.length) {
    report.status = "incompatible";
    return report;
  }
  for (const [index, id] of ids.entries()) {
    for (const field of rectFields) {
      const left = native.geometry[id][field], right = browser.observations[index].rect[field];
      if (!Object.is(left, right)) {
        differences.push({ id, field, left, right, path: "$.observations[" + index + "].rect." + field });
      }
    }
  }
  // A hidden node has no box at all. Compare visibility as well as border
  // geometry, then compare the restored state independently of the baseline.
  for (const phase of phases) {
    for (const id of ids) {
      const left = native.reflow[phase][id], right = browser.reflow[phase][id];
      if (left === null || right === null) {
        if (left !== right) {
          differences.push({ kind: "visibility", phase, id, field: "visible",
            left: left !== null, right: right !== null,
            path: "$.reflow." + phase + "." + id });
        }
        continue;
      }
      for (const field of rectFields) {
        if (!Object.is(left[field], right[field])) {
          differences.push({ kind: "geometry", phase, id, field,
            left: left[field], right: right[field],
            path: "$.reflow." + phase + "." + id + "." + field });
        }
      }
    }
  }
  for (const id of heightIds) {
    for (const field of rectFields) {
      const left = native.percentageHeights[id][field];
      const right = browser.percentageHeights[id][field];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "percentage-height", id, field, left, right,
          path: "$.percentageHeights." + id + "." + field,
        });
      }
    }
  }
  for (const id of borderIds) {
    for (const field of rectFields) {
      const left = native.borderBoxes[id][field];
      const right = browser.borderBoxes[id][field];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "border-box", id, field, left, right,
          path: "$.borderBoxes." + id + "." + field,
        });
      }
    }
  }
  for (const id of widthBoundIds) {
    for (const field of rectFields) {
      const left = native.widthBounds[id][field];
      const right = browser.widthBounds[id][field];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "width-bound", id, field, left, right,
          path: "$.widthBounds." + id + "." + field,
        });
      }
    }
  }
  for (const id of heightBoundIds) {
    for (const field of rectFields) {
      const left = native.heightBounds[id][field];
      const right = browser.heightBounds[id][field];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "height-bound", id, field, left, right,
          path: "$.heightBounds." + id + "." + field,
        });
      }
    }
  }
  for (const phase of visibilityPhases) {
    const a = native.visibilityStates[phase], b = browser.visibilityStates[phase];
    for (const id of visibilityIds) {
      for (const field of rectFields) {
        const left = a.boxes[id][field], right = b.boxes[id][field];
        if (!Object.is(left, right)) {
          differences.push({
            kind: "visibility-box", phase, id, field, left, right,
            path: "$.visibilityStates." + phase + ".boxes." + id + "." + field,
          });
        }
      }
      if (!Object.is(a.computed[id], b.computed[id])) {
        differences.push({
          kind: "visibility-computed", phase, id,
          left: a.computed[id], right: b.computed[id],
          path: "$.visibilityStates." + phase + ".computed." + id,
        });
      }
    }
    for (const probe of visibilityProbes) {
      const left = a.hits[probe], right = b.hits[probe];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "visibility-hit", phase, probe, left, right,
          path: "$.visibilityStates." + phase + ".hits." + probe,
        });
      }
    }
  }
  for (const phase of clipPhases) {
    const a = native.descendantClipStates[phase];
    const b = browser.descendantClipStates[phase];
    if (a.mode !== b.mode) {
      differences.push({
        kind: "descendant-clip-mode", phase, left: a.mode, right: b.mode,
        path: "$.descendantClipStates." + phase + ".mode",
      });
    }
    for (const id of clipIds) {
      for (const field of rectFields) {
        const left = a.boxes[id][field], right = b.boxes[id][field];
        if (!Object.is(left, right)) {
          differences.push({
            kind: "descendant-clip-box", phase, id, field, left, right,
            path: "$.descendantClipStates." + phase + ".boxes." + id + "." + field,
          });
        }
      }
    }
    for (const probe of clipProbes) {
      const left = a.hits[probe], right = b.hits[probe];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "descendant-clip-hit", phase, probe, left, right,
          path: "$.descendantClipStates." + phase + ".hits." + probe,
        });
      }
    }
  }
  for (const phase of boxlessPhases) {
    const a = native.boxlessStates[phase];
    const b = browser.boxlessStates[phase];
    for (const field of ["display", "hasBox"]) {
      if (!Object.is(a[field], b[field])) {
        differences.push({
          kind: "boxless-mode", phase, field,
          left: a[field], right: b[field],
          path: "$.boxlessStates." + phase + "." + field,
        });
      }
    }
    for (const id of boxlessIds) {
      for (const field of rectFields) {
        const left = a.boxes[id][field], right = b.boxes[id][field];
        if (!Object.is(left, right)) {
          differences.push({
            kind: "boxless-box", phase, id, field, left, right,
            path: "$.boxlessStates." + phase + ".boxes." + id + "." + field,
          });
        }
      }
    }
    for (const probe of boxlessProbes) {
      const left = a.hits[probe], right = b.hits[probe];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "boxless-hit", phase, probe, left, right,
          path: "$.boxlessStates." + phase + ".hits." + probe,
        });
      }
    }
  }
  for (const phase of pointerPhases) {
    const a = native.pointerStates[phase], b = browser.pointerStates[phase];
    for (const id of pointerIds) {
      for (const field of rectFields) {
        const left = a.boxes[id][field], right = b.boxes[id][field];
        if (!Object.is(left, right)) {
          differences.push({
            kind: "pointer-box", phase, id, field, left, right,
            path: "$.pointerStates." + phase + ".boxes." + id + "." + field,
          });
        }
      }
      if (!Object.is(a.computed[id], b.computed[id])) {
        differences.push({
          kind: "pointer-computed", phase, id,
          left: a.computed[id], right: b.computed[id],
          path: "$.pointerStates." + phase + ".computed." + id,
        });
      }
    }
    for (const probe of pointerProbes) {
      const left = a.hits[probe], right = b.hits[probe];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "pointer-hit", phase, probe, left, right,
          path: "$.pointerStates." + phase + ".hits." + probe,
        });
      }
    }
  }
  for (const phase of autoMarginPhases) {
    const a = native.autoMarginStates[phase], b = browser.autoMarginStates[phase];
    for (const id of autoMarginIds) {
      for (const field of rectFields) {
        const left = a.boxes[id][field], right = b.boxes[id][field];
        if (!Object.is(left, right)) {
          differences.push({
            kind: "auto-margin-box", phase, id, field, left, right,
            path: "$.autoMarginStates." + phase + ".boxes." + id + "." + field,
          });
        }
      }
    }
    for (const id of autoMarginChildren) {
      for (const side of ["left", "right"]) {
        const left = a.computed[id][side], right = b.computed[id][side];
        if (!Object.is(left, right)) {
          differences.push({
            kind: "auto-margin-computed", phase, id, side, left, right,
            path: "$.autoMarginStates." + phase + ".computed." + id + "." + side,
          });
        }
      }
    }
    for (const probe of autoMarginProbes) {
      const left = a.hits[probe], right = b.hits[probe];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "auto-margin-hit", phase, probe, left, right,
          path: "$.autoMarginStates." + phase + ".hits." + probe,
        });
      }
    }
  }
  for (const phase of relativePhases) {
    const a = native.relativeStates[phase], b = browser.relativeStates[phase];
    for (const id of relativeIds) {
      for (const field of rectFields) {
        const left = a.boxes[id][field], right = b.boxes[id][field];
        if (!Object.is(left, right)) {
          differences.push({
            kind: "relative-box", phase, id, field, left, right,
            path: "$.relativeStates." + phase + ".boxes." + id + "." + field,
          });
        }
      }
    }
    for (const id of relativeStyled) {
      for (const field of ["position", "left", "top"]) {
        const left = a.computed[id][field], right = b.computed[id][field];
        if (!Object.is(left, right)) {
          differences.push({
            kind: "relative-computed", phase, id, field, left, right,
            path: "$.relativeStates." + phase + ".computed." + id + "." + field,
          });
        }
      }
    }
    for (const probe of relativeProbes) {
      const left = a.hits[probe], right = b.hits[probe];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "relative-hit", phase, probe, left, right,
          path: "$.relativeStates." + phase + ".hits." + probe,
        });
      }
    }
  }

  for (const phase of percentPhases) {
    const a = native.percentStates[phase], b = browser.percentStates[phase];
    if (!Object.is(a.scrollY, b.scrollY)) {
      differences.push({
        kind: "percent-margin-scroll", phase, left: a.scrollY, right: b.scrollY,
        path: "$.percentStates." + phase + ".scrollY",
      });
    }
    for (const id of percentIds) {
      for (const field of rectFields) {
        const left = a.boxes[id][field], right = b.boxes[id][field];
        if (!Object.is(left, right)) {
          differences.push({
            kind: "percent-margin-box", phase, id, field, left, right,
            path: "$.percentStates." + phase + ".boxes." + id + "." + field,
          });
        }
      }
    }
    for (const side of percentSides) {
      const left = a.computed[side], right = b.computed[side];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "percent-margin-computed", phase, side, left, right,
          path: "$.percentStates." + phase + ".computed." + side,
        });
      }
    }
    for (const probe of percentProbes) {
      const left = a.hits[probe], right = b.hits[probe];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "percent-margin-hit", phase, probe, left, right,
          path: "$.percentStates." + phase + ".hits." + probe,
        });
      }
    }
  }
  const nativeScroll = native.scrollViewport, browserScroll = browser.scrollViewport;
  if (!Object.is(nativeScroll.documentHeight, browserScroll.documentHeight)) {
    differences.push({
      kind: "viewport-scroll", field: "documentHeight",
      left: nativeScroll.documentHeight, right: browserScroll.documentHeight,
      path: "$.scrollViewport.documentHeight",
    });
  }
  for (const phase of scrollPhases) {
    const left = nativeScroll[phase], right = browserScroll[phase];
    for (const field of ["scrollX", "scrollY", "hitTarget"]) {
      if (!Object.is(left[field], right[field])) {
        differences.push({
          kind: "viewport-scroll", phase, field, left: left[field], right: right[field],
          path: "$.scrollViewport." + phase + "." + field,
        });
      }
    }
    for (const field of rectFields) {
      if (!Object.is(left.targetRect[field], right.targetRect[field])) {
        differences.push({
          kind: "viewport-scroll", phase, field: "targetRect." + field,
          left: left.targetRect[field], right: right.targetRect[field],
          path: "$.scrollViewport." + phase + ".targetRect." + field,
        });
      }
    }
  }
  // These targets come from independent source-side elementFromPoint and
  // native hit_test calls. A missing/outside target is explicit JSON null.
  for (const phase of targetPhases) {
    for (const probe of probes) {
      const left = native.hitTargets[phase][probe];
      const right = browser.hitTargets[phase][probe];
      if (left !== right) {
        differences.push({
          kind: "hit-target", phase, probe, left, right,
          path: "$.hitTargets." + phase + "." + probe,
        });
      }
    }
  }
  report.status = differences.length ? "different" : "equal";
  return report;
}

async function main(args) {
  if (args.length !== 2) throw new Error("usage: node tests/browser-oracles/native-block-bridge.mjs native.json browser.json");
  const [native, browser, fixture] = await Promise.all([
    ...args.map(path => readFile(path, "utf8").then(JSON.parse)),
    readFile(new URL("./native-block-bridge.html", import.meta.url)),
  ]);
  const report = compareNativeBlockBridge(native, browser, fixture);
  process.stdout.write(JSON.stringify(report, null, 2) + "\n");
  process.exitCode = report.status === "equal" ? 0 : report.status === "different" ? 1 : 2;
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(String(error) + "\n");
    process.exitCode = 2;
  });
}
