// One Vitest run for the whole repo (`pnpm test`). Rust has its own tests (`cargo test`).
import { defineConfig } from "vitest/config"

export default defineConfig({
  test: {
    // The first resvg call loads a native binary: 8 s on a cold CI runner, over the 5 s default.
    testTimeout: 30_000,
    include: ["{packages,apps,examples,scripts}/**/*.test.ts"],
    exclude: ["**/node_modules/**", "**/dist/**", "**/target/**"],
  },
})
