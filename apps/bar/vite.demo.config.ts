// Builds the real bar UI for the website (ADR 0037): the same React code as the app, running on
// mocked IPC instead of Rust. Output goes to the website's public/demo, which the landing page frames.
import path from "path"
import tailwindcss from "@tailwindcss/vite"
import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"

export default defineConfig({
  root: path.resolve(import.meta.dirname, "demo"),
  base: "./",
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(import.meta.dirname, "./src") } },
  build: {
    outDir: path.resolve(import.meta.dirname, "../web/public/demo"),
    emptyOutDir: true,
  },
})
