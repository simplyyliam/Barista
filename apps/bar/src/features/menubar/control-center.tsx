import { commands, events } from "@/shared/bindings";
import { useSnapshot } from "@/shared/hooks/use-snapshot";
import { Icon, SlidersHorizontalIcon } from "@barista/ui/icons";
import { useOpenPanel } from "@/features/panel";
import {
  CONTROL_CENTER_HEIGHT,
  CONTROL_CENTER_WIDTH,
} from "@/features/panel/control-center/control-center-panel";
import { useItemBounds } from "./hooks/use-item-bounds";
import { MenubarItem } from "./menubar-item";

const ITEM_ID = "system/control-center";

/**
 * The control center icon, last in the bar (App.png). Opens the panel under it, or Windows' own
 * Quick Settings when the "windows" mode is set.
 */
export function ControlCenter() {
  const open = useOpenPanel();
  const mode = useSnapshot(commands.getControlCenterMode, (h) =>
    events.controlCenterModeChanged.listen((e) => h(e.payload)),
  );

  // The panel opens under the item, so Rust needs to know where it is drawn.
  useItemBounds(ITEM_ID);

  const isOpen = open?.anchor === ITEM_ID;

  const click = () => {
    if (mode === "windows") {
      void commands.openWindowsControlCenter();
    } else {
      void commands.openPanel("control-center", ITEM_ID, CONTROL_CENTER_WIDTH, CONTROL_CENTER_HEIGHT, "none");
    }
  };

  return (
    <MenubarItem
      role="button"
      tabIndex={0}
      aria-label="Control center"
      aria-expanded={isOpen}
      data-item-id={ITEM_ID}
      onClick={click}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          click();
        }
      }}
      open={isOpen}
      className="cursor-default outline-none focus-visible:bg-bar-hover"
    >
      <Icon icon={SlidersHorizontalIcon} size={16} />
    </MenubarItem>
  );
}
