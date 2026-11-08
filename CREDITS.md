# Credits

Barista is built on the work of these open-source projects. Every library a `package.json` or
`Cargo.toml` in this repo depends on has a row here; `scripts/credits.test.ts` fails if one is missing.
The first column must be the exact package or crate name.

## Rust

| Name              | Version     | License           | Link                                      |
| ----------------- | ----------- | ----------------- | ----------------------------------------- |
| tauri             | 2.12        | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri       |
| tauri-build       | 2.7         | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri       |
| tauri-specta      | 2.0.0-rc.26 | MIT               | https://github.com/specta-rs/tauri-specta |
| specta            | 2.0.0-rc.26 | MIT               | https://github.com/specta-rs/specta       |
| specta-typescript | 0.0.13      | MIT               | https://github.com/specta-rs/specta       |
| specta-serde      | 0.0.13      | MIT               | https://github.com/specta-rs/specta       |
| serde             | 1.0         | MIT OR Apache-2.0 | https://github.com/serde-rs/serde         |
| serde_json        | 1.0         | MIT OR Apache-2.0 | https://github.com/serde-rs/json          |

## JavaScript and TypeScript

| Name                      | Version | License           | Link                                                   |
| ------------------------- | ------- | ----------------- | ------------------------------------------------------ |
| react                     | 19.3    | MIT               | https://github.com/facebook/react                      |
| react-dom                 | 19.3    | MIT               | https://github.com/facebook/react                      |
| @tauri-apps/api           | 2.12    | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri                    |
| @tauri-apps/cli           | 2.12    | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri                    |
| vite                      | 8.3     | MIT               | https://github.com/vitejs/vite                         |
| @vitejs/plugin-react      | 6.2     | MIT               | https://github.com/vitejs/vite-plugin-react            |
| tailwindcss               | 4.3     | MIT               | https://github.com/tailwindlabs/tailwindcss            |
| @tailwindcss/vite         | 4.3     | MIT               | https://github.com/tailwindlabs/tailwindcss            |
| typescript                | 6.0     | Apache-2.0        | https://github.com/microsoft/TypeScript                |
| @types/node               | 24      | MIT               | https://github.com/DefinitelyTyped/DefinitelyTyped     |
| @types/react              | 19.2    | MIT               | https://github.com/DefinitelyTyped/DefinitelyTyped     |
| @types/react-dom          | 19.2    | MIT               | https://github.com/DefinitelyTyped/DefinitelyTyped     |
| eslint                    | 10.14   | MIT               | https://github.com/eslint/eslint                       |
| @eslint/js                | 10.0    | MIT               | https://github.com/eslint/eslint                       |
| typescript-eslint         | 8.72    | MIT               | https://github.com/typescript-eslint/typescript-eslint |
| eslint-plugin-react-hooks | 7.1     | MIT               | https://github.com/facebook/react                      |
| globals                   | 17.14   | MIT               | https://github.com/sindresorhus/globals                |
| prettier                  | 3.9     | MIT               | https://github.com/prettier/prettier                   |
| vitest                    | 5.1     | MIT               | https://github.com/vitest-dev/vitest                   |

## Design tooling

| Name      | Version  | License | Link                            |
| --------- | -------- | ------- | ------------------------------- |
| shadcn/ui | CLI 4.23 | MIT     | https://github.com/shadcn-ui/ui |

shadcn/ui is configured (`apps/bar/components.json`) but no components are installed yet.
