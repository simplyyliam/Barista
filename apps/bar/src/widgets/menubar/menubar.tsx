import { Battery, Clock, DisplayDate, MenubarItemGroup, StatusItems, Volume } from "@/features/menubar"
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
    <div className="grid h-full grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center justify-between px-bar-x font-bar text-bar text-bar-fg">
      <MenubarItemGroup className="justify-self-start">
        <p>Barista {version}</p>
      </MenubarItemGroup>
      <MenubarItemGroup className="justify-self-center">
        <DisplayDate/>
      </MenubarItemGroup>
      <MenubarItemGroup className="justify-self-end">
        <StatusItems />
        {/*<Volume />*/}
        <Clock />
      </MenubarItemGroup>
    </div>
  )
}
