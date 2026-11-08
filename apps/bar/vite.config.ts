import path from "path"
import tailwindcss from "@tailwindcss/vite"
import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(import.meta.dirname, "./src") } },
  // Tauri: keep Rust errors visible, and use a fixed port that matches devUrl in
  // src-tauri/tauri.conf.json. strictPort fails loudly instead of moving to another port.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Cargo rebuilds Rust itself; Vite doesn't need to watch it.
    watch: { ignored: ["**/src-tauri/**"] },
  },
})
