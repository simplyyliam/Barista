// Static website: `next build` writes plain files to out/ (no server, see ADR 0004).
import { createMDX } from "fumadocs-mdx/next"

/** @type {import('next').NextConfig} */
const config = {
  output: "export",
  trailingSlash: true,
  reactStrictMode: true,
  // Image optimisation needs a server, so images are served as they are.
  images: { unoptimized: true },
}

export default createMDX()(config)
