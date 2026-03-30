# SRECon Hybrid Web Deck

Browser-native talk deck for the SRECon `jsoncompat` presentation.

## Commands

```bash
cd deck
npm install
npm run dev
```

Build the SPA:

```bash
npm run build
```

The main website build also publishes this deck at `https://jsoncompat.com/deck/`.

Export PDF or slide PNGs:

```bash
npm run export:pdf
npm run export:png
```

Capture simulator backup stills and GIFs:

```bash
npm run capture:assets
```

This writes generated artifacts under `deck/artifacts/`.

## Layout

- `slides.md`: slide flow plus speaker notes
- `components/SimulatorDeck.vue`: Slidev wrapper around the existing Pixi simulator
- `styles/index.css`: custom theme styling
- `scripts/capture-simulator-assets.mjs`: Playwright + `ffmpeg` backup capture flow
