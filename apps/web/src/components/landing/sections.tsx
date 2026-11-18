import { Check, Plus, AppIcon, WindowsGlyph } from "./icons"
import { Shot } from "./shot"
import { DOWNLOAD_URL, FAQ } from "./site"

function Download({ light = false }: { light?: boolean }) {
  return (
    <a href={DOWNLOAD_URL} className={`btn btn--lg ${light ? "btn--light" : "btn--dark"}`}>
      <WindowsGlyph />
      Download for Windows
    </a>
  )
}

export function Hero() {
  return (
    <section className="section" id="top">
      <div className="sheet sheet--split">
        <div className="split">
          <div className="split__text stack">
            <p className="eyebrow" data-reveal>A menu bar for Windows.</p>
            <h1 className="h1" data-reveal data-delay="80">Your apps, served at the top.</h1>
            <p className="lead" data-reveal data-delay="160">
              Barista puts your tray icons, notifications and quick controls in one clean bar along
              the top of every screen, and reserves the space like macOS.
            </p>
            <div data-reveal data-delay="240"><Download /></div>
            <div className="trust" data-reveal data-delay="320">
              <span>Free to start</span>
              <span>Windows 11</span>
              <span>Idles near 0% CPU</span>
            </div>
          </div>
          <div className="split__media bleed-r" data-reveal data-delay="200">
            <Shot label="The bar on a desktop" size="Screenshot, 1000 × 700" ratio="10/7" className="shot--r" />
          </div>
        </div>
      </div>
    </section>
  )
}

export function Statement() {
  return (
    <section className="section" style={{ padding: "40px var(--gutter)" }}>
      <p className="statement text-cut" data-reveal>
        Windows gave you a taskbar. Barista adds a menu bar: tray icons, notifications and quick
        controls in one quiet line at the top of every screen.
      </p>
    </section>
  )
}

const TILES = [
  { t: "Tray icons", p: "Every app from the Windows tray lives in the bar. Click one and its own menu opens right under it.", s: "Bar with tray icons" },
  { t: "Notifications", p: "Read, clear and open notifications from a panel under the clock.", s: "Notification panel" },
  { t: "Control center", p: "Wi-Fi, Bluetooth, brightness, volume and now playing in one card.", s: "Control center" },
  { t: "Every monitor", p: "Pick the screen for the bar. It follows your display scale and keeps its space reserved.", s: "Bar on two monitors" },
]

export function Features() {
  return (
    <section className="section" id="features">
      <div className="sheet center">
        <div className="stack center">
          <p className="eyebrow" data-reveal>Tray, notifications &amp; controls</p>
          <h2 className="h2" data-reveal data-delay="80">Everything, one click up.</h2>
          <p className="lead" style={{ maxWidth: 720 }} data-reveal data-delay="160">
            The things you reach for all day, moved out of the corner of the screen and into one
            place that stays put.
          </p>
        </div>
        <div className="bento">
          {TILES.map((x, i) => (
            <div className="tile" key={x.t} data-reveal data-delay={i * 80}>
              <h3 className="h3">{x.t}</h3>
              <p>{x.p}</p>
              <Shot label={x.s} size="Screenshot, 16:9" ratio="16/9" />
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}

export function Sdk() {
  return (
    <section className="section" id="sdk">
      <div className="sheet sheet--split">
        <div className="split">
          <div className="split__media bleed-l" data-reveal>
            <div className="code" role="img" aria-label="Example code using the Barista SDK">
              <pre>
                <span className="c">{"// npm install @baristabar/sdk"}</span>
                {"\n"}
                <span className="k">import</span> * <span className="k">as</span> BA <span className="k">from</span> {'"@baristabar/sdk"'}
                {"\n\n"}
                <span className="k">const</span> item = <span className="k">await</span> BA.StatusItem.create({"{"}
                {"\n  "}id: {'"counter"'},
                {"\n  "}label: {'"0"'},
                {"\n"}
                {"})"}
                {"\n\n"}
                item.on({'"click"'}, () =&gt; item.setLabel({'"1"'}))
              </pre>
            </div>
          </div>
          <div className="split__text stack">
            <p className="eyebrow" data-reveal>An SDK for your app</p>
            <h2 className="h2" data-reveal data-delay="80">Your app, in the menu bar.</h2>
            <p className="lead" data-reveal data-delay="160">
              Add a status item with a few lines. Send a menu and the bar draws it, or open your own
              window as a popover under the icon.
            </p>
            <div className="pills" data-reveal data-delay="240">
              {["Status items", "Menus", "Popovers", "Reconnects by itself", "Typed protocol"].map((p) => (
                <span className="pill" key={p}>{p}</span>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}

const LIGHT = [
  { n: "~0%", t: "CPU at idle", p: "The bar waits for events from Windows. Nothing polls." },
  { n: "1 line", t: "Space reserved", p: "Windows are laid out below the bar, like on macOS, not under it." },
  { n: "Any DPI", t: "Sharp everywhere", p: "Sizes are worked out per monitor, so it stays crisp on mixed screens." },
]

export function Light() {
  return (
    <section className="section on-dark">
      <div className="wrap center">
        <div className="stack center">
          <p className="eyebrow" style={{ color: "rgba(255,255,255,.4)" }} data-reveal>Built for all-day use</p>
          <h2 className="h2" data-reveal data-delay="80">Light by design.</h2>
        </div>
        <div className="cards">
          {LIGHT.map((c, i) => (
            <div className="card" key={c.t} data-reveal data-delay={i * 80}>
              <span className="stat">{c.n}</span>
              <h3 className="h3">{c.t}</h3>
              <p>{c.p}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}

export function Twin() {
  return (
    <section className="section">
      <div className="twin">
        <div className="sheet" data-reveal>
          <div className="stack">
            <p className="eyebrow">Settings</p>
            <h2 className="h2">Make it yours.</h2>
            <p className="lead">Bar height, monitor, appearance and what starts with Windows, in one small window.</p>
          </div>
          <div className="shot-bottom"><Shot label="Settings window" size="Screenshot, 4:3" ratio="4/3" className="shot--bottom" /></div>
        </div>
        <div className="sheet" data-reveal data-delay="120">
          <div className="stack">
            <p className="eyebrow">Your call</p>
            <h2 className="h2">Switch back any time.</h2>
            <p className="lead">Everything Barista replaces has a Windows mode. Flip it and the bar steps aside.</p>
          </div>
          <div className="shot-bottom"><Shot label="Barista or Windows switches" size="Screenshot, 4:3" ratio="4/3" className="shot--bottom" /></div>
        </div>
      </div>
    </section>
  )
}

function List({ items }: { items: string[] }) {
  return (
    <ul className="checks">
      {items.map((i) => (
        <li key={i}><Check />{i}</li>
      ))}
    </ul>
  )
}

export function Pricing() {
  return (
    <section className="section on-dark" id="pricing">
      <div className="wrap center">
        <div className="stack center">
          <h2 className="h2" data-reveal>Free. Pro for $5.</h2>
          <p className="lead" data-reveal data-delay="80">Start free. Upgrade once if you want more, and keep it for good.</p>
        </div>
        <div className="pricing">
          <div className="price-card" data-reveal>
            <div className="price-card__head"><AppIcon /><h3 className="h3">Free</h3></div>
            <div className="price-row"><span className="price">$0</span></div>
            <List items={["The menu bar on every monitor", "Tray icons and notifications", "Control center", "The SDK for your own apps"]} />
            <Download />
          </div>
          <div className="price-card" data-reveal data-delay="120">
            <div className="price-card__head"><AppIcon /><h3 className="h3">Pro</h3></div>
            <div className="price-row"><span className="price">$5</span><span className="price-unit">one-off</span></div>
            <div className="note">One-time payment. No subscription.</div>
            <List items={["Everything in Free", "Pro features coming soon"]} />
            <span className="btn btn--lg" aria-disabled="true">Coming soon</span>
          </div>
        </div>
        <p className="fine fine--dim" style={{ marginTop: 24 }} data-reveal>Pro is not on sale yet. You will not be charged for anything today.</p>
      </div>
    </section>
  )
}

export function FinalCta() {
  return (
    <section className="section">
      <div className="sheet center" style={{ paddingBottom: 0 }}>
        <div className="stack center">
          <h2 className="h2" data-reveal>Give Windows a menu bar.</h2>
          <p className="lead" style={{ maxWidth: 560 }} data-reveal data-delay="80">
            One download. Your tray, your notifications and your controls, all at the top.
          </p>
          <div data-reveal data-delay="160"><Download /></div>
        </div>
        <div className="shot-bottom" data-reveal data-delay="240">
          <Shot label="The bar on a desktop" size="Screenshot, 1000 × 600" ratio="5/3" className="shot--bottom" />
        </div>
      </div>
    </section>
  )
}

export function Faq() {
  return (
    <section className="section on-dark" id="faq">
      <div className="wrap faq-grid">
        <div className="faq-intro" data-reveal>
          <h2 className="h2">Good to know.</h2>
          <p className="lead">The short answers about the taskbar, your tray and your PC.</p>
          <a href="/docs/" className="btn btn--lg btn--ghost">Read the docs</a>
        </div>
        <div className="faq">
          {FAQ.map((f, i) => (
            <details className="faq__item text-cut" key={f.q} data-reveal data-delay={i * 60}>
              <summary className="faq__q"><Plus />{f.q}</summary>
              <p className="faq__a">{f.a}</p>
            </details>
          ))}
        </div>
      </div>
    </section>
  )
}
