# Interactive JSONCompat Wire Simulator

Live, keyboard-driven visualization of schema evolution and on-wire compatibility.

## Run

```bash
cd interactive
pnpm install
pnpm run dev
```

Open the local Vite URL (usually `http://localhost:5173`).

## Controls

- `Right Arrow` / `N`: next state
- `Left Arrow` / `P`: previous state
- `Space`: pause/resume
- `R`: reset to initial state
- `D`: toggle debug state/version overlay

## Behavior

- Packets flow continuously forever.
- Schema changes happen on keypress.
- Existing in-flight packets are never rewritten during a transition.
- That preserves realistic transient compatibility failures when upgrading schemas.

## Story Source

The default scenario lives in:

- `story/default-story.json`

To customize the demo, edit this file (or point the app at another story JSON).

## Test

```bash
pnpm test
pnpm run build
```
