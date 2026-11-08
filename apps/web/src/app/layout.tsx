import "./global.css"
import { RootProvider } from "fumadocs-ui/provider/next"
import type { Metadata } from "next"
import type { ReactNode } from "react"

export const metadata: Metadata = {
  title: { default: "Barista", template: "%s | Barista" },
  description: "Your apps, served at the top.",
}

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="flex min-h-screen flex-col">
        {/* Search is off until v0.10: static export needs a prebuilt index (ADR 0004). */}
        <RootProvider search={{ enabled: false }}>{children}</RootProvider>
      </body>
    </html>
  )
}
