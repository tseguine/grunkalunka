// Fixed assets contain no document bytes, path, session token or derived view.
export const livePage = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <meta name="referrer" content="no-referrer">
  <title>Note outline · Grunkalunka</title>
  <link rel="stylesheet" href="/style.css">
  <script type="module" src="/client.mjs"></script>
</head>
<body>
  <main>
    <header>
      <p class="brand">Grunkalunka <span>Live session</span></p>
      <h1>Note outline</h1>
      <p class="introduction">A live view of your Markdown note.</p>
    </header>
    <section class="view" aria-label="Outline view">
      <div class="surface">
        <canvas id="note-frame" width="320" height="160" role="img" tabindex="0"
          aria-label="Note outline view" aria-disabled="true" aria-busy="true"
          aria-describedby="session-status session-help">The note view requires canvas support.</canvas>
      </div>
      <div class="view-actions">
        <button id="reload-note" type="button" disabled aria-describedby="reload-help">Reload note</button>
        <p id="reload-help">After editing the file, reload to see your changes.</p>
      </div>
      <p id="session-status" role="status" aria-live="polite" aria-atomic="true">Connecting to your note…</p>
    </section>
    <p id="session-help">Scroll to see more. With the view focused, use the arrow
      or Page Up/Down keys; Home and End jump to the top or bottom. Select the
      outline title to show or hide headings, or press Enter or Space while the title is visible.</p>
    <noscript>JavaScript is required to display this live view.</noscript>
  </main>
</body>
</html>
`;

export const liveStyles = `
:root { color-scheme: light; font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #263732; background: #f4f6f2; }
* { box-sizing: border-box; }
body { margin: 0; min-height: 100vh; }
main { width: min(100% - 40px, 736px); margin: clamp(32px, 8vh, 80px) auto; }
.brand { display: flex; align-items: center; gap: 12px; margin: 0 0 24px; font-size: 13px; font-weight: 650; letter-spacing: .025em; }
.brand span { padding: 4px 8px; color: #3d6356; background: #e4ece5; border: 1px solid #d2dfd4; border-radius: 5px; font-size: 11px; font-weight: 550; }
h1 { margin: 0; font-size: clamp(28px, 5vw, 38px); line-height: 1.2; letter-spacing: -.03em; font-weight: 600; }
.introduction { margin: 10px 0 28px; color: #627168; line-height: 1.5; }
.view { padding: clamp(16px, 4vw, 32px); background: #fff; border: 1px solid #dce3db; border-radius: 12px; box-shadow: 0 8px 32px #26373207; }
.surface { background: #fff; }
canvas { display: block; width: 100%; max-width: 640px; height: auto; margin: 0 auto; padding: 0; border: 0; image-rendering: pixelated; outline: none; }
canvas[role="button"]:not([aria-disabled="true"]) { cursor: pointer; }
canvas:focus-visible { outline: 3px solid #2a7760; outline-offset: 5px; }
.view-actions { display: flex; align-items: center; flex-wrap: wrap; gap: 10px 14px; margin-top: 22px; }
button { padding: 8px 13px; border: 1px solid #2a7760; border-radius: 6px; background: #2a7760; color: #fff; font: inherit; font-size: 13px; cursor: pointer; }
button:focus-visible { outline: 3px solid #2a7760; outline-offset: 3px; }
button:disabled { background: #edf1ed; color: #768078; border-color: #dce3db; cursor: default; }
#reload-help { margin: 0; color: #66766b; font-size: 12px; line-height: 1.5; }
#session-status { margin: 22px 0 0; min-height: 20px; color: #53665b; font-size: 13px; line-height: 1.5; }
#session-help, noscript { display: block; margin: 18px 2px 0; max-width: 58ch; color: #66766b; font-size: 13px; line-height: 1.65; }
@media (max-width: 420px) { main { width: calc(100% - 24px); } .brand { gap: 8px; } }
`;
