import { defineConfig } from "vite";
import preact from "@preact/preset-vite";

// @ts-ignore: process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [preact()],

  // Modern build target for Windows WebView2
  build: {
    target: "esnext",
    minify: "esbuild",
  },

  resolve: {
    // Force a single version of Preact to be bundled
    dedupe: ["preact"],
  },

  // Vite options tailored for Tauri development
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
