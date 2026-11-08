// Placeholder so `pnpm dev` shows something. Liam replaces this with his own structure.
import { useEffect, useState } from "react"
import { commands } from "@/shared/bindings"

export default function App() {
  const [version, setVersion] = useState("…")

  useEffect(() => {
    // Proves the Rust <-> TypeScript bridge works.
    void commands.getVersion().then(setVersion)
  }, [])

  return <p className="p-4 font-sans">Barista {version}</p>
}
