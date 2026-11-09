# Credits

Barista is built on the work of these open-source projects. Every library a `package.json` or
`Cargo.toml` in this repo depends on has a row here; `scripts/credits.test.ts` fails if one is missing.
The first column must be the exact package or crate name.

## Rust

| Name              | Version     | License           | Link                                          |
| ----------------- | ----------- | ----------------- | --------------------------------------------- |
| tauri             | 2.12        | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri           |
| tauri-build       | 2.7         | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri           |
| tauri-specta      | 2.0.0-rc.26 | MIT               | https://github.com/specta-rs/tauri-specta     |
| specta            | 2.0.0-rc.26 | MIT               | https://github.com/specta-rs/specta           |
| specta-typescript | 0.0.13      | MIT               | https://github.com/specta-rs/specta           |
| specta-serde      | 0.0.13      | MIT               | https://github.com/specta-rs/specta           |
| serde             | 1.0         | MIT OR Apache-2.0 | https://github.com/serde-rs/serde             |
| serde_json        | 1.0         | MIT OR Apache-2.0 | https://github.com/serde-rs/json              |
| tokio             | 1.54        | MIT               | https://github.com/tokio-rs/tokio             |
| base64            | 0.23        | MIT OR Apache-2.0 | https://github.com/marshallpierce/rust-base64 |
| windows           | 0.62        | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs       |

## JavaScript and TypeScript

| Name                       | Version | License           | Link                                                   |
| -------------------------- | ------- | ----------------- | ------------------------------------------------------ |
| react                      | 19.3    | MIT               | https://github.com/facebook/react                      |
| react-dom                  | 19.3    | MIT               | https://github.com/facebook/react                      |
| @tauri-apps/api            | 2.12    | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri                    |
| @tauri-apps/cli            | 2.12    | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri                    |
| vite                       | 8.3     | MIT               | https://github.com/vitejs/vite                         |
| @vitejs/plugin-react       | 6.2     | MIT               | https://github.com/vitejs/vite-plugin-react            |
| tailwindcss                | 4.3     | MIT               | https://github.com/tailwindlabs/tailwindcss            |
| @tailwindcss/vite          | 4.3     | MIT               | https://github.com/tailwindlabs/tailwindcss            |
| typescript                 | 6.0     | Apache-2.0        | https://github.com/microsoft/TypeScript                |
| @types/node                | 24      | MIT               | https://github.com/DefinitelyTyped/DefinitelyTyped     |
| @types/react               | 19.2    | MIT               | https://github.com/DefinitelyTyped/DefinitelyTyped     |
| @types/react-dom           | 19.2    | MIT               | https://github.com/DefinitelyTyped/DefinitelyTyped     |
| eslint                     | 10.14   | MIT               | https://github.com/eslint/eslint                       |
| @eslint/js                 | 10.0    | MIT               | https://github.com/eslint/eslint                       |
| typescript-eslint          | 8.72    | MIT               | https://github.com/typescript-eslint/typescript-eslint |
| eslint-plugin-react-hooks  | 7.1     | MIT               | https://github.com/facebook/react                      |
| globals                    | 17.14   | MIT               | https://github.com/sindresorhus/globals                |
| prettier                   | 3.9     | MIT               | https://github.com/prettier/prettier                   |
| vitest                     | 5.1     | MIT               | https://github.com/vitest-dev/vitest                   |
| @tailwindcss/postcss       | 4.3     | MIT               | https://github.com/tailwindlabs/tailwindcss            |
| @base-ui/react             | 1.9     | MIT               | https://github.com/mui/base-ui                         |
| @hugeicons/react           | 1.1     | MIT               | https://github.com/hugeicons/hugeicons                 |
| @hugeicons/core-free-icons | 4.4     | MIT               | https://github.com/hugeicons/hugeicons                 |
| @fontsource-variable/inter | 5.3     | OFL-1.1           | https://github.com/fontsource/font-files               |
| class-variance-authority   | 0.7     | Apache-2.0        | https://github.com/joe-bell/cva                        |
| cn                         | 0.5     | MIT               | https://github.com/shadcn-ui/cn                        |
| next                       | 16.4    | MIT               | https://github.com/vercel/next.js                      |
| fumadocs-core              | 16.18   | MIT               | https://github.com/fuma-nama/fumadocs                  |
| fumadocs-ui                | 16.18   | MIT               | https://github.com/fuma-nama/fumadocs                  |
| fumadocs-mdx               | 15.5    | MIT               | https://github.com/fuma-nama/fumadocs                  |
| @types/mdx                 | 2.0     | MIT               | https://github.com/DefinitelyTyped/DefinitelyTyped     |
| tw-animate-css             | 1.4     | MIT               | https://github.com/Wombosvideo/tw-animate-css          |

## Design tooling

| Name   | Version | License | Link                            |
| ------ | ------- | ------- | ------------------------------- |
| shadcn | 4.23    | MIT     | https://github.com/shadcn-ui/ui |

shadcn/ui lives in the shared `packages/ui` package (preset `base-mira`, built on Base UI). Only the Button it generates is installed so far.
