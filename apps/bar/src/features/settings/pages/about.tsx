import { useEffect, useState } from "react"
import { commands } from "@/shared/bindings"
import { cn } from "cn"
import { Banner } from "../about/banner"
import { HIGHLIGHTS, RELEASE } from "../about/whats-new"
import { PageTitle } from "../ui"

/** What Barista is, and what is new in this version. Centred, with the spotlight banner on top. */
export function AboutPage() {
  const [version, setVersion] = useState("")
  const [picked, setPicked] = useState(HIGHLIGHTS[0]!.id)
  const [plays, setPlays] = useState(0)
  useEffect(() => {
    void commands.getVersion().then(setVersion)
  }, [])
  const current = HIGHLIGHTS.find((h) => h.id === picked) ?? HIGHLIGHTS[0]!

  return (
    <div className="flex flex-col gap-6">
      <PageTitle title="About" />
      <Banner scene={current.scene} play={`${current.id}-${plays}`} />

      <div className="flex flex-col items-center gap-1 text-center">
        <h3 className="text-[13px] font-medium text-foreground">Barista</h3>
        <p className="text-xs text-muted-foreground/70 tabular-nums">Version {version || "..."}</p>
        <p className="max-w-sm pt-1 text-xs text-muted-foreground">
          A menu bar for Windows, like the one on a Mac. Tray icons live in it, and apps can add their own items with
          the Barista SDK.
        </p>
      </div>

      <section aria-label="What's new" className="flex flex-col items-center gap-1">
        <h3 className="pb-1 text-[11px] text-muted-foreground/70">What&apos;s new in {RELEASE}</h3>
        {HIGHLIGHTS.map((item) => (
          <button
            key={item.id}
            type="button"
            aria-pressed={item.id === picked}
            onClick={() => {
              setPicked(item.id)
              setPlays((n) => n + 1)
            }}
            className={cn(
              "flex w-full max-w-md flex-col items-center gap-0.5 rounded-lg px-3 py-2 text-center outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/40",
              item.id === picked ? "bg-white/[0.06]" : "hover:bg-white/[0.03]",
            )}
          >
            <span className="text-[13px] text-foreground">{item.title}</span>
            <span className="text-xs text-muted-foreground/70">{item.text}</span>
          </button>
        ))}
      </section>

      <p className="text-center text-[11px] text-muted-foreground/50">
        Built with the help of open-source libraries. Their licences are listed in CREDITS.md.
      </p>
    </div>
  )
}
