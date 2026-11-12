import { useState } from "react"
import { cn } from "cn"
import { Calendar } from "@barista/ui/components/ui/calendar"
import { Icon, LucideChevronDown } from "@barista/ui/icons"
import { useClockSettings } from "./hooks/use-system-state"

const WEEKDAYS = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]

/** "Sunday, 08 November", the card heading. */
function headingDate(date: Date) {
  const weekday = date.toLocaleDateString("en-US", { weekday: "long" })
  const month = date.toLocaleDateString("en-US", { month: "long" })
  return `${weekday}, ${String(date.getDate()).padStart(2, "0")} ${month}`
}

/** The "Sunday, 08 November" card of App.png: today's date as the heading, shadcn's calendar below. */
export function CalendarCard() {
  const clock = useClockSettings()
  const [open, setOpen] = useState(true)
  const today = new Date()

  return (
    <section
      aria-label="Calendar"
      className="w-panel-w rounded-panel bg-panel p-panel-inset text-panel-fg"
    >
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className="flex w-full items-center justify-between rounded-panel-child px-2 py-3 text-panel-title outline-none focus-visible:ring-2 focus-visible:ring-panel-fg/50"
      >
        <span>{headingDate(today)}</span>
        <Icon icon={LucideChevronDown} size={16} className={cn("transition-transform", !open && "-rotate-90")} />
      </button>

      {open && (
        <Calendar
          // The first weekday comes from Windows, the same setting the clock service reads.
          weekStartsOn={(clock?.firstWeekday ?? 0) as 0 | 1 | 2 | 3 | 4 | 5 | 6}
          showOutsideDays
          // Always six rows, like the design, so the card does not jump between months.
          fixedWeeks
          formatters={{
            formatWeekdayName: (date) => WEEKDAYS[date.getDay()] ?? "",
            // The design only says "November"; the year shows once you leave this year.
            formatCaption: (date) =>
              date.toLocaleDateString("en-US", {
                month: "long",
                ...(date.getFullYear() !== today.getFullYear() && { year: "numeric" }),
              }),
          }}
          className="mt-1 w-full rounded-panel-child bg-panel-raised p-3 text-panel-fg [--cell-radius:9999px] [--cell-size:--spacing(7)]"
          classNames={{
            // Each value below replaces the shadcn default for that part, so the layout classes stay.
            month_caption: "flex h-(--cell-size) w-full items-center justify-start px-1",
            caption_label: "text-[13px] font-normal select-none",
            nav: "absolute inset-x-0 top-0 flex h-(--cell-size) w-full items-center justify-end gap-1",
            button_previous: "size-5 p-0 text-panel-fg-muted select-none hover:bg-panel-separator hover:text-panel-fg",
            button_next: "size-5 p-0 text-panel-fg-muted select-none hover:bg-panel-separator hover:text-panel-fg",
            weekday: "flex-1 text-[11px] font-normal text-panel-fg-muted select-none",
            week: "mt-1 flex w-full",
            // Fixed-size round cells (the default is a square as wide as the column).
            day: "group/day relative flex h-8 w-full items-center justify-center p-0 text-center text-[12px] select-none [&_button]:size-7 [&_button]:min-w-0 [&_button]:rounded-full [&_button]:text-[12px] [&_button]:font-normal",
            today: "[&_button]:bg-panel-fg [&_button]:text-panel [&_button]:hover:bg-panel-fg [&_button]:hover:text-panel",
            outside: "text-panel-fg-muted [&_button]:text-panel-fg-muted",
          }}
        />
      )}
    </section>
  )
}
