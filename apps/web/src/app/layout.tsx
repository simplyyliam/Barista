import "./global.css"
import "@/components/landing/landing.css"
import { RootProvider } from "fumadocs-ui/provider/next"
import type { Metadata } from "next"
import { Inter } from "next/font/google"
import type { ReactNode } from "react"

// opsz axis gives the tight "Inter Display" cut the landing page uses for headings.
const inter = Inter({ subsets: ["latin"], axes: ["opsz"], variable: "--font-inter", display: "swap" })

export const metadata: Metadata = {
  title: { default: "Barista", template: "%s | Barista" },
  description: "Your apps, served at the top.",
}

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={inter.variable} suppressHydrationWarning>
      <body className="flex min-h-screen flex-col">
        {/* Search is off until v0.10: static export needs a prebuilt index (ADR 0004). */}
        {/* RootProvider wraps next-themes: the docs follow the system theme (switch in the docs header,
            hotkey "d"). The landing page sets its own light colours on .sl, so it does not change. */}
        <RootProvider
          search={{ enabled: false }}
          theme={{ attribute: "class", defaultTheme: "system", enableSystem: true, disableTransitionOnChange: true }}
        >
          {children}
        </RootProvider>
      </body>
    </html>
  )
}
