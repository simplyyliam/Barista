// PLACEHOLDER: Liam designs and builds the real landing page.
import Link from "next/link"

export default function Home() {
  return (
    <main className="flex flex-1 flex-col items-center justify-center gap-4 p-8 text-center">
      <h1 className="text-4xl font-semibold">Barista</h1>
      <p className="text-muted-foreground">Your apps, served at the top.</p>
      <button
        type="button"
        disabled
        className="rounded-md bg-primary px-4 py-2 text-primary-foreground opacity-50"
      >
        Download
      </button>
      <Link href="/docs" className="text-sm underline">
        Docs
      </Link>
    </main>
  )
}
