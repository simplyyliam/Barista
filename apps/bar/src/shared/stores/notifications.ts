import {
  commands,
  events,
  type Mode,
  type Notifications_Deserialize,
} from "@/shared/bindings"
import { useSnapshot } from "@/shared/hooks/use-snapshot"

/** The notifications Rust read from Windows, grouped by app. `null` until the first answer. */
export const useNotifications = () =>
  useSnapshot<Notifications_Deserialize>(commands.getNotifications, (h) =>
    events.notificationsChanged.listen((e) => h(e.payload)),
  )

/** Who draws the notification panel: "barista" (ours) or "windows" (the v0.8 setting flips it). */
export const useNotificationMode = () =>
  useSnapshot<Mode>(commands.getNotificationMode, (h) =>
    events.notificationModeChanged.listen((e) => h(e.payload)),
  )
