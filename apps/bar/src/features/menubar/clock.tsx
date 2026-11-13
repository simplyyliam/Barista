import { commands } from "@/shared/bindings";
import { useNotificationMode, useNotifications } from "@/shared/stores/notifications";
import { useOpenPanel } from "@/features/panel";
import { useClockSettings } from "@/shared/hooks/use-system-state";
import { useItemBounds } from "./hooks/use-item-bounds";
import { useNow } from "./hooks/use-now";
import { formatClock } from "./lib/format-clock";
import { MenubarItem } from "./menubar-item";

const ITEM_ID = "system/clock";

/**
 * Date and time. Clicking it opens the notification panel (or Windows' own notification center
 * when the "windows" mode is set); a dot shows unread notifications.
 */
export function Clock() {
  const now = useNow();
  const settings = useClockSettings();
  const mode = useNotificationMode();
  const notifications = useNotifications();
  const open = useOpenPanel();

  // The panel opens under the item, so Rust needs to know where it is drawn.
  useItemBounds(ITEM_ID);

  const { date, time } = formatClock(now, settings?.hour24 ?? false);
  const isOpen = open?.anchor === ITEM_ID;
  const unread = mode === "barista" && notifications?.unread === true;

  const click = () => {
    if (mode === "windows") {
      void commands.openWindowsNotifications();
    } else {
      void commands.openPanel("notifications", ITEM_ID, 300, 400, "none");
    }
  };

  return (
    <MenubarItem
      role="button"
      tabIndex={0}
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
      className="relative cursor-default gap-1.5 tabular-nums outline-none focus-visible:bg-bar-hover"
    >
      <span>{date}</span>
      {/* Hidden until the 12/24 h setting is known, so a 24 hour clock never flashes "1:18 PM". */}
      <span className={settings ? undefined : "invisible"}>{time}</span>
      {unread && (
        <span
          role="status"
          aria-label="Unread notifications"
          className="absolute top-1 left-1.5 size-1.5 rounded-full bg-bar-fg"
        />
      )}
    </MenubarItem>
  );
}
