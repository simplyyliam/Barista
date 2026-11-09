import { commands } from "@/shared/bindings"
import { useState, useEffect } from "react"

export default function Menubar() {
  const [version, setVersion] = useState("…")

  useEffect(() => {
    // Proves the Rust <-> TypeScript bridge works.
    void commands.getVersion().then(setVersion)
  }, [])

  // Placeholder: fills the bar's height and centres its content vertically (tokens.css).
  return (
    <p className="flex h-full items-center px-bar-x font-bar text-bar text-bar-fg">
      Barista {version}
    </p>
  )
}