import { Fragment } from "react"
import { Clock, MenubarItemGroup, StatusItems, TrayItems } from "@/features/menubar"
import { DEFAULT_ITEMS, renderBuiltIn, zoneOf } from "@/features/menubar/built-in"
import { useItemSpace } from "@/features/menubar/hooks/use-item-space"
import { useResetHover } from "@/features/menubar/hooks/use-reset-hover"
import { useApplySettings } from "@/shared/hooks/use-apply-settings"
import { useItems } from "@/shared/hooks/use-items"
import { Motion } from "@/shared/motion"
import { useSettings } from "@/shared/stores/settings"

export default function Menubar() {
  useApplySettings()
  useResetHover()
  // Rust splits the items into bar and overflow by the room the bar has.
  useItemSpace(useItems().map((item) => item.id))

  // What to show and in which order is the user's choice (settings window). Until Rust answers,
  // the default list keeps the bar from flashing empty.
  const settings = useSettings()
  const shown = (settings?.menuBar.items ?? DEFAULT_ITEMS).filter((item) => item.visible)
  const left = shown.filter((item) => zoneOf(item) === "left")
  const right = shown.filter((item) => zoneOf(item) === "right")
  const dateInCenter = settings?.menuBar.clock.datePosition === "center" && right.some((item) => item.id === "clock")

  // Fills the bar's height and centres the pills vertically. Sizes: --menubar-* in tokens.css.
  return (
    <Motion>
      <div
        data-bar-root
        className="grid h-full grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center justify-between px-menubar-edge font-bar text-bar font-bar-item text-bar-fg"
      >
        {/* gap 0: neighbouring hover pills touch, like macOS */}
        <MenubarItemGroup className="justify-self-start">
          {left.map((item) => (
            <Fragment key={item.id}>{renderBuiltIn(item)}</Fragment>
          ))}
        </MenubarItemGroup>
        <MenubarItemGroup className="justify-self-center" data-bar-center>
          {dateInCenter && <Clock part="date" />}
        </MenubarItemGroup>
        <MenubarItemGroup className="justify-self-end gap-menubar-status-gap" data-bar-right>
          <TrayItems />
          <StatusItems />
          {right.map((item) =>
            item.id === "clock" && dateInCenter ? (
              <Clock key={item.id} part="time" />
            ) : (
              <Fragment key={item.id}>{renderBuiltIn(item)}</Fragment>
            ),
          )}
        </MenubarItemGroup>
      </div>
    </Motion>
  )
}
