import { commands } from "@/shared/bindings";
import { useNotificationMode, useNotifications } from "@/shared/stores/notifications";
import { useOpenPanel } from "@/features/panel";
import { AnimatePresence, motion } from "motion/react";
import { ICON_HIDDEN, ICON_SHOWN, ICON_TRANSITION } from "@/shared/motion";
import { useClockSettings } from "@/shared/hooks/use-system-state";
import { useSettings } from "@/shared/stores/settings";
import { useItemBounds } from "./hooks/use-item-bounds";
import { useNow } from "./hooks/use-now";
import { formatClock } from "./lib/format-clock";
import { MenubarItem } from "./menubar-item";

const CLOCK_ID = "system/clock";
const DATE_ID = "system/date";

/**
 * Date and time. Clicking it opens the notification panel (or Windows' own notification center
 * when the "windows" mode is set); a dot shows unread notifications.
 */
export function Clock({ part = "full" }: { part?: "full" | "time" | "date" }) {
  const itemId = part === "date" ? DATE_ID : CLOCK_ID;
  const prefs = useSettings()?.menuBar.clock;
  const now = useNow();
  const settings = useClockSettings();
  const mode = useNotificationMode();
  const notifications = useNotifications();
  const open = useOpenPanel();

  // The panel opens under the item, so Rust needs to know where it is drawn.
  useItemBounds(itemId);

  const { date, time } = formatClock(now, settings?.hour24 ?? false, prefs);
  const isOpen = open?.anchor === itemId;
  const unread = mode === "barista" && notifications?.unread === true;

  const click = () => {
    if (mode === "windows") {
      void commands.openWindowsNotifications();
    } else {
      void commands.openPanel("notifications", itemId, 300, 400, "none");
    }
  };

  return (
    <MenubarItem
      role="button"
      tabIndex={0}
      aria-expanded={isOpen}
      data-item-id={itemId}
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
      {part !== "time" && date && <span>{date}</span>}
      {/* Hidden until the 12/24 h setting is known, so a 24 hour clock never flashes "1:18 PM". */}
      {part !== "date" && <span className={settings ? undefined : "invisible"}>{time}</span>}
      <AnimatePresence initial={false}>
        {unread && (
          <motion.span
            key="unread"
            role="status"
            aria-label="Unread notifications"
            initial={ICON_HIDDEN}
            animate={ICON_SHOWN}
            exit={ICON_HIDDEN}
            transition={ICON_TRANSITION}
            className="absolute top-1 left-1.5 size-1.5 rounded-full bg-bar-fg"
          />
        )}
      </AnimatePresence>
    </MenubarItem>
  );
}
