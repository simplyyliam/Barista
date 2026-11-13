import { useEffect, useState } from "react"
import { commands } from "@/shared/bindings"
import { Icon, Settings01Icon, ShutDownIcon } from "@barista/ui/icons"
import { useUser } from "./hooks/use-control-center"

const CONFIRM_MS = 5000

const circle = "flex size-10 items-center justify-center outline-none focus-visible:ring-2 focus-visible:ring-cc-fg/60"
// Buttons sit 5 px inside their group, so their radius is the group's minus 5 (ADR 0008).
const buttonRadius = "rounded-[max(0px,calc(var(--radius-panel-child)-var(--spacing-cc-button-pad)))]"

function initials(name: string) {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join("")
}

/**
 * The top row (50 px): who is signed in, then the settings and power buttons. Power asks first:
 * the row turns into "Shut down this PC?" and goes back on its own after 5 seconds, or when the
 * panel closes.
 */
export function UserHeader({ active }: { active: boolean }) {
  const user = useUser()
  const [confirming, setConfirming] = useState(false)

  useEffect(() => {
    if (!active) setConfirming(false)
  }, [active])

  useEffect(() => {
    if (!confirming) return
    const timer = setTimeout(() => setConfirming(false), CONFIRM_MS)
    return () => clearTimeout(timer)
  }, [confirming])

  if (confirming) {
    return (
      <div className="flex h-cc-header items-center gap-2 rounded-panel-child bg-cc-tile p-cc-button-pad text-cc-fg">
        <span className="min-w-0 flex-1 truncate pl-3 text-[13px] font-medium">Shut down this PC?</span>
        <button
          type="button"
          autoFocus
          onClick={() => setConfirming(false)}
          className={`h-10 px-4 text-[13px] ${buttonRadius} bg-cc-tile outline-none hover:bg-cc-tile-hover focus-visible:ring-2 focus-visible:ring-cc-fg/60`}
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={() => {
            setConfirming(false)
            void commands.runAction("shutDown", true)
          }}
          className={`h-10 px-4 text-[13px] font-medium ${buttonRadius} bg-cc-active outline-none focus-visible:ring-2 focus-visible:ring-cc-fg/60`}
        >
          Shut down
        </button>
      </div>
    )
  }

  const name = user?.available ? user.name : ""
  return (
    <div className="flex h-cc-header gap-1.5">
      <div className="flex min-w-0 flex-1 items-center gap-2.5 rounded-panel-child bg-cc-tile py-cc-avatar-pad pr-cc-avatar-pad-end pl-cc-avatar-pad text-cc-fg">
        <div className="flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-[max(0px,calc(var(--radius-panel-child)-var(--spacing-cc-avatar-pad)))] bg-cc-tile text-[13px] font-medium">
          {user?.avatar ? (
            <img src={user.avatar} alt="" draggable={false} className="size-full object-cover" />
          ) : name ? (
            initials(name)
          ) : null}
        </div>
        <div className="flex min-w-0 flex-col leading-tight">
          <span className="truncate text-[11px] font-medium">{name}</span>
          {user?.email && <span className="truncate text-[9px] text-cc-fg-muted">{user.email}</span>}
        </div>
      </div>
      <div className="flex shrink-0 items-center rounded-panel-child bg-cc-tile p-cc-button-pad text-cc-fg">
        <button
          type="button"
          aria-label="Windows settings"
          onClick={() => void commands.openSettings("home")}
          className={`${circle} ${buttonRadius} hover:bg-cc-tile-hover`}
        >
          <Icon icon={Settings01Icon} size={18} />
        </button>
        <button
          type="button"
          aria-label="Shut down"
          onClick={() => setConfirming(true)}
          className={`${circle} ${buttonRadius} hover:bg-cc-tile-hover`}
        >
          <Icon icon={ShutDownIcon} size={18} />
        </button>
      </div>
    </div>
  )
}
