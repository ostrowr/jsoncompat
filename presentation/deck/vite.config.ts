import path from "node:path";
import { defineConfig } from "vite";

const presentationRoot = path.resolve(__dirname, "..");
const pixiLegacyModule = path.resolve(
  presentationRoot,
  "interactive/node_modules/pixi.js-legacy/dist/pixi-legacy.mjs",
);

export default defineConfig({
  resolve: {
    alias: {
      "@interactive": path.resolve(presentationRoot, "interactive/src"),
      "pixi.js-legacy": pixiLegacyModule,
    },
  },
  server: {
    fs: {
      allow: [presentationRoot],
    },
  },
});
