# 0004: Website with Next.js + Fumadocs (static export)

Status: accepted (2026-11-08, website setup)

## Context

Barista needs a website: a landing page (Liam designs it) and SDK/protocol docs (content in v0.10).
It lives in `apps/web`, shares the shadcn theme from `packages/ui`, and has no server: it is
hosted as plain files.

## Options

|                                  | Next.js + Fumadocs        | Astro Starlight  | VitePress  |
| -------------------------------- | ------------------------- | ---------------- | ---------- |
| Latest checked                   | next 16.4, fumadocs 16.18 | -                | -          |
| React + shadcn/ui + Tailwind v4  | **yes**, same as the bar  | via islands only | no (Vue)   |
| Landing page and docs in one app | **yes**                   | yes              | docs-first |
| Static export                    | `output: 'export'`        | default          | default    |
| Reuses `@barista/ui`             | **yes**                   | awkward          | no         |

## Decision

Use **Next.js (App Router) + Fumadocs** in `apps/web`, package `@barista/web`, with
`output: 'export'` (writes a static `out/` folder).

- Docs content is MDX in `apps/web/content/docs`, served at `/docs`.
- Collections are declared in code with `fumadocs-mdx/macro`: no `source.config.ts`, no codegen step.
- No server features: no route handlers, middleware, image optimisation or ISR.
- Fumadocs' built-in search needs a static index (`staticGET` + `useStaticSearch`); it is **off**
  for now and gets switched on in v0.10 with the real docs.

## Consequences

- Anything that needs a server (forms, analytics endpoints) must be a third-party service.
- Hosting is out of scope here; any static host can serve `out/`.
- Fumadocs majors follow Next majors; upgrade `fumadocs-*` together.
