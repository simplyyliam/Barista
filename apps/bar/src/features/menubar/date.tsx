import { cn } from "cn";
import { commands } from "@/shared/bindings";
import { useNotificationMode, useNotifications } from "@/shared/stores/notifications";
import { useOpenPanel } from "@/features/panel";
import { useItemBounds } from "./hooks/use-item-bounds";
import { useNow } from "./hooks/use-now";
import { MenubarItem } from "./menubar-item";

const ITEM_ID = "system/date";

/** "Sunday, 11 May" */
function formatDate(date: Date) {
  const weekday = date.toLocaleDateString("en-GB", { weekday: "long" });
  const month = date.toLocaleDateString("en-GB", { month: "long" });
  return `${weekday}, ${date.getDate()} ${month}`;
}

/**
 * The date in the middle of the bar. Clicking it opens the notification panel (or Windows' own
 * notification center when the "windows" mode is set). A dot shows unread notifications.
 */
export function DateItem() {
  const now = useNow();
  const mode = useNotificationMode();
  const notifications = useNotifications();
  const open = useOpenPanel();

  // The panel opens under the item, so Rust needs to know where it is drawn.
  useItemBounds(ITEM_ID);

  const isOpen = open?.anchor === ITEM_ID;
  const unread = mode === "barista" && notifications?.unread === true;

  const click = () => {
    if (mode === "windows") {
      void commands.openWindowsNotifications();
    } else {
      void commands.openPanel("notifications", ITEM_ID, 340, 400, "none");
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
      className={cn("relative cursor-default gap-1.5 rounded-md outline-none focus-visible:bg-bar-hover", isOpen && "bg-bar-hover")}
    >
      <p>{formatDate(now)}</p>
      {unread && (
        <span
          role="status"
          aria-label="Unread notifications"
          className="size-1.5 rounded-full bg-bar-fg"
        />
      )}
    </MenubarItem>
  );
}
