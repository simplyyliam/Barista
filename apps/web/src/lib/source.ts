// Reads the MDX pages in content/docs. The macro needs no source.config.ts or codegen.
import { loader } from "fumadocs-core/source"
import { defineDocs } from "fumadocs-mdx/macro"

const docs = defineDocs({ dir: "content/docs" })

export const source = loader({
  baseUrl: "/docs",
  source: docs.toFumadocsSource(),
})
