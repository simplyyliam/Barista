import { Battery, Clock, StatusItems, Volume } from "@/features/menubar"
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
    <div className="flex h-full items-center justify-between px-bar-x font-bar text-bar text-bar-fg">
      <p>Barista {version}</p>
      <div className="flex items-center justify-center gap-4">
        <StatusItems/>
        <Volume />
        <Battery />
        <Clock />
      </div>
    </div>
  )
}
