import tailwindcss from "@tailwindcss/vite";
import viteReact from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";

import { resolve } from "node:path";
import { TanStackRouterVite } from "@tanstack/router-plugin/vite";
import { cloudflare } from "@cloudflare/vite-plugin";

// https://vitejs.dev/config/
export default defineConfig({
  base: "", // make asset URLs relative so they work regardless of hosting sub‑path
  plugins: [
    TanStackRouterVite({ autoCodeSplitting: true }),
    viteReact(),
    tailwindcss(),
    wasm(),
    cloudflare(),
  ],
  build: {
    target: "esnext",
  },
  resolve: {
    alias: {
      "@": resolve(__dirname, "./src"),
    },
  },
});
