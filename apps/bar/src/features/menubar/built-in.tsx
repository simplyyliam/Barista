import type { ReactNode } from "react"
import type { BarItemPref_Serialize } from "@/shared/bindings"
import { AppMenus } from "./app-menus"
import { AppName } from "./app-name"
import { Battery } from "./battery"
import { BrandItem } from "./brand-item"
import { Clock } from "./clock"
import { ControlCenter } from "./control-center"
import { Search } from "./search"
import { Volume } from "./volume"
import { Wifi } from "./wifi"

/** The bar's own items, by the ids Rust saves (`settings.rs` BAR_ITEMS). */
export const BUILT_IN_IDS = ["brand", "app-name", "app-menus", "battery", "wifi", "volume", "search", "control-center", "clock"] as const
export type BuiltInId = (typeof BUILT_IN_IDS)[number]

/** Which side an item lives on. The order in the settings list decides the order inside a side. */
export const ZONE: Record<BuiltInId, "left" | "right"> = {
  brand: "left",
  "app-name": "left",
  "app-menus": "left",
  battery: "right",
  wifi: "right",
  volume: "right",
  search: "right",
  "control-center": "right",
  clock: "right",
}

/** Names for the settings window. */
export const BUILT_IN_NAMES: Record<BuiltInId, string> = {
  brand: "Barista logo",
  "app-name": "App name",
  "app-menus": "App menus",
  battery: "Battery",
  wifi: "Wi-Fi",
  volume: "Volume",
  search: "Search",
  "control-center": "Control center",
  clock: "Clock",
}

export const DEFAULT_ITEMS: BarItemPref_Serialize[] = BUILT_IN_IDS.map((id) => ({ id, visible: true, display: "both" }))

export const isBuiltIn = (id: string): id is BuiltInId => (BUILT_IN_IDS as readonly string[]).includes(id)

/** One item of the bar for its saved preferences. */
export function renderBuiltIn(pref: BarItemPref_Serialize): ReactNode {
  if (!isBuiltIn(pref.id)) return null
  switch (pref.id) {
    case "brand":
      return <BrandItem />
    case "app-name":
      return <AppName />
    case "app-menus":
      return <AppMenus />
    case "battery":
      return <Battery showLabel={pref.display !== "icon"} />
    case "wifi":
      return <Wifi />
    case "volume":
      return <Volume />
    case "search":
      return <Search />
    case "control-center":
      return <ControlCenter />
    case "clock":
      return <Clock />
  }
}

export const zoneOf = (pref: BarItemPref_Serialize): "left" | "right" => (isBuiltIn(pref.id) ? ZONE[pref.id] : "right")
