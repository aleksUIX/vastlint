import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  plugins: [react(), wasm()],
  build: {
    target: "esnext",
  },
  optimizeDeps: {
    exclude: ["vastlint", "vastlint-client", "vastlint-react"],
  },
  resolve: {
    preserveSymlinks: true,
  },
  server: {
    fs: {
      allow: [path.resolve(__dirname, "..", "..")],
    },
  },
});