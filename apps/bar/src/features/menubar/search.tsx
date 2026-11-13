import { SearchIcon } from "./icons"
import { MenubarItem } from "./menubar-item"

/** Search icon. No action yet: the bar has no search to open. */
export function Search({ open }: { open?: boolean }) {
  return (
    <MenubarItem open={open} className="cursor-default" aria-label="Search">
      <SearchIcon />
    </MenubarItem>
  )
}
