// One Vitest run for the whole repo (`pnpm test`). Rust has its own tests (`cargo test`).
import { defineConfig } from "vitest/config"

export default defineConfig({
  test: {
    include: ["{packages,apps,examples,scripts}/**/*.test.ts"],
    exclude: ["**/node_modules/**", "**/dist/**", "**/target/**"],
  },
})
