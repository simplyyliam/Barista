import { MenubarItem } from "./menubar-item";

export function DisplayDate() {
  const date = new Date()
  return (
    <MenubarItem>
      {date.toLocaleDateString("en-us", {
        weekday: "long",
        month: "long",
        day: "numeric"
      })}
    </MenubarItem>
  )
}
