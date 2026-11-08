import { commands } from "@/shared/bindings"
import { useState, useEffect } from "react"

export default function Menubar() {
  const [version, setVersion] = useState("…")

  useEffect(() => {
    // Proves the Rust <-> TypeScript bridge works.
    void commands.getVersion().then(setVersion)
  }, [])

  return <p className="p-4 font-sans">Barista {version}</p>
}