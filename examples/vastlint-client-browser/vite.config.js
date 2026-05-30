import { defineConfig } from "vite";
import path from "node:path";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  plugins: [wasm()],
  build: {
    target: "esnext",
  },
  optimizeDeps: {
    exclude: ["vastlint", "vastlint-client"],
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