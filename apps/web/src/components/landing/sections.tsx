import { Bean, BeanGlyph, Plus, WindowsGlyph } from "./icons"
import { DOWNLOAD_URL, FAQ } from "./site"

function Download({ white = false }: { white?: boolean }) {
  return (
    <a href={DOWNLOAD_URL} className={`btn btn--lg ${white ? "btn--white" : "btn--primary"}`}>
      <WindowsGlyph />
      Download for Windows
    </a>
  )
}

/* ---- recreations of the real bar UI (no personal data) ---- */

function Notifications() {
  return (
    <div className="ui">
      <div className="ui__head"><span>Notifications</span><span>Clear all</span></div>
      <div className="note"><span className="note__ic" /><div><b>Messages</b><span>You may have a new message</span></div></div>
      <div className="note"><span className="note__ic" /><div><b>Snipping Tool</b><span>Screenshot copied to clipboard. Saved to your screenshots folder.</span></div></div>
      <div className="note"><span className="note__ic" /><div><b>Calendar</b><span>Design review starts in 10 minutes</span></div></div>
    </div>
  )
}

function ControlCenter() {
  return (
    <div className="ui">
      <div className="tiles">
        <div className="tl on"><i /></div>
        <div className="tl"><i /></div>
        <div className="tl on"><i /></div>
        <div className="tl"><i /></div>
      </div>
      <div className="slider"><b style={{ width: "62%" }} /></div>
      <div className="slider"><b style={{ width: "38%" }} /></div>
    </div>
  )
}

function Seg({ on }: { on: "Barista" | "Windows" }) {
  return (
    <span className="seg">
      <span className={on === "Barista" ? "on" : ""}>Barista</span>
      <span className={on === "Windows" ? "on" : ""}>Windows</span>
    </span>
  )
}

/* ---- sections ---- */

export function Hero() {
  return (
    <section className="section" id="top" style={{ paddingTop: "calc(var(--bar-h) + 90px)" }}>
      <div className="wrap split">
        <div className="split__text stack">
          <p className="eyebrow" data-reveal>Menu bar for Windows</p>
          <h1 className="h1" data-reveal data-delay="80">
            Your apps, served at the <span className="accent">top</span>.
          </h1>
          <p className="lead" data-reveal data-delay="160">
            Barista puts your tray icons, notifications and quick controls in one clean bar along
            the top of every screen, and keeps the space for it like macOS does.
          </p>
          <div className="row" data-reveal data-delay="240">
            <Download />
            <a href="/docs/" className="btn btn--lg btn--quiet">Read the docs</a>
          </div>
          <div className="facts" data-reveal data-delay="320">
            <span>Free to start</span>
            <span>Windows 11</span>
            <span>Idles near 0% CPU</span>
          </div>
        </div>
        <div className="split__media hero__media" style={{ position: "relative" }} data-reveal data-delay="200">
          <Bean style={{ width: 130, top: -112, right: 56, transform: "rotate(8deg)" }} />
          <div className="scene" role="img" aria-label="The Barista bar on a desktop, with notifications and the control center open">
            <div className="mbar">
              <span>Explorer</span>
              <span>Sunday, 11 May</span>
              <span style={{ display: "flex", alignItems: "center" }}><i /><i className="sq" /><i />&nbsp;&nbsp;22:00</span>
            </div>
            <div className="scene__body">
              <Notifications />
              <ControlCenter />
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}

export function Statement() {
  return (
    <section className="section">
      <p className="statement text-cut" data-reveal>
        Windows gave you a taskbar. Barista adds a menu bar: tray icons, notifications and quick
        controls in one quiet line at the top of every screen.
      </p>
    </section>
  )
}

export function Features() {
  return (
    <section className="section" id="features">
      <div className="wrap">
        <div className="stack center center" style={{ textAlign: "center" }}>
          <p className="eyebrow" data-reveal>Tray, notifications &amp; controls</p>
          <h2 className="h2" data-reveal data-delay="80">Everything, one click up.</h2>
          <p className="lead" style={{ maxWidth: 680 }} data-reveal data-delay="160">
            The things you reach for all day, moved out of the corner of the screen and into one
            place that stays put.
          </p>
        </div>
        <div className="bento">
          <div className="tile tile--wide" data-reveal>
            <h3 className="h3">Tray icons</h3>
            <p>Every app from the Windows tray lives in the bar. Click one and its own menu opens right under it.</p>
            <div className="tile__media">
              <div className="trayrow" aria-hidden="true"><i /><i /><i /><i /><i /><span>22:00</span></div>
            </div>
          </div>
          <div className="tile tile--tall" data-reveal data-delay="80">
            <h3 className="h3">Notifications</h3>
            <p>Read, clear and open them from a panel under the clock.</p>
            <div className="tile__media"><Notifications /></div>
          </div>
          <div className="tile" data-reveal data-delay="160">
            <h3 className="h3">Control center</h3>
            <p>Try the switches. They are real ones.</p>
            <div className="tile__media toggles">
              <label className="tg">Wi-Fi<input type="checkbox" defaultChecked /><i /></label>
              <label className="tg">Bluetooth<input type="checkbox" defaultChecked /><i /></label>
              <label className="tg">Focus<input type="checkbox" /><i /></label>
            </div>
          </div>
          <div className="tile" data-reveal data-delay="240">
            <h3 className="h3">Every monitor</h3>
            <p>Pick the screen for the bar. It follows your display scale.</p>
            <div className="tile__media monitors" aria-hidden="true">
              <span className="mon" /><span className="mon mon--s" />
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}

export function Sdk() {
  return (
    <section className="section" id="sdk">
      <div className="wrap split">
        <div className="split__media" data-reveal>
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
          <div className="tags" data-reveal data-delay="240">
            {["Status items", "Menus", "Popovers", "Reconnects by itself", "Typed protocol"].map((p) => (
              <span className="tag" key={p} style={{ background: "#fff" }}>{p}</span>
            ))}
          </div>
        </div>
      </div>
    </section>
  )
}

export function Light() {
  const S = [
    { n: "~0%", t: "CPU at idle", p: "The bar waits for events from Windows. Nothing polls." },
    { n: "1 bar", t: "Space reserved", p: "Windows are laid out below the bar, like on macOS, not under it." },
    { n: "Any DPI", t: "Sharp everywhere", p: "Sizes are worked out per monitor, so it stays crisp on mixed screens." },
  ]
  return (
    <section className="section">
      <div className="wrap">
        <div className="stack">
          <p className="eyebrow" data-reveal>Built for all-day use</p>
          <h2 className="h2" data-reveal data-delay="80">Light on your PC.</h2>
        </div>
        <div className="stats">
          {S.map((s, i) => (
            <div className="stat" key={s.t} data-reveal data-delay={i * 80}>
              <span className="bignum">{s.n}</span>
              <h3 className="h3">{s.t}</h3>
              <p>{s.p}</p>
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
      <div className="wrap twin">
        <div className="card" data-reveal>
          <div className="stack">
            <p className="eyebrow">Settings</p>
            <h2 className="h2">Make it yours.</h2>
            <p className="lead">Bar height, monitor, appearance and what starts with Windows, in one small window.</p>
          </div>
          <div className="card__media" role="img" aria-label="Settings window preview">
            <div className="set">
              <div className="set__row">Bar height<span className="set__slider"><b style={{ width: "40%" }} /></span></div>
              <div className="set__row">Window corners<span className="set__slider"><b style={{ width: "70%" }} /></span></div>
              <div className="set__row">Start with Windows<span className="seg"><span className="on">On</span><span>Off</span></span></div>
            </div>
          </div>
        </div>
        <div className="card" data-reveal data-delay="120">
          <div className="stack">
            <p className="eyebrow">Your call</p>
            <h2 className="h2">Switch back any time.</h2>
            <p className="lead">Everything Barista replaces has a Windows mode. Flip it and the bar steps aside.</p>
          </div>
          <div className="card__media" role="img" aria-label="Barista or Windows switches">
            <div className="set">
              <div className="set__row">Notifications<Seg on="Barista" /></div>
              <div className="set__row">Control center<Seg on="Windows" /></div>
              <div className="set__row">Tray<Seg on="Barista" /></div>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}

function List({ items }: { items: string[] }) {
  return (
    <ul className="checks">
      {items.map((i) => (
        <li key={i}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3 8.5 3.2 3.2L13 4.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
          {i}
        </li>
      ))}
    </ul>
  )
}

export function Pricing() {
  return (
    <section className="section" id="pricing">
      <div className="wrap">
        <div className="stack center" style={{ textAlign: "center" }}>
          <h2 className="h2" data-reveal>Free. Pro for $5.</h2>
          <p className="lead" data-reveal data-delay="80">Start free. Upgrade once if you want more, and keep it for good.</p>
        </div>
        <div className="pricing">
          <div className="price-card" data-reveal>
            <div className="price-card__head"><span className="appicon"><BeanGlyph /></span><h3 className="h3">Free</h3></div>
            <div className="price-row"><span className="bignum">$0</span></div>
            <List items={["The menu bar on every monitor", "Tray icons and notifications", "Control center", "The SDK for your own apps"]} />
            <Download />
          </div>
          <div className="price-wrap" data-reveal data-delay="120">
            <Bean style={{ width: 96, top: -76, right: 40, transform: "rotate(-10deg)" }} />
            <div className="price-card">
              <div className="price-card__head"><span className="appicon"><BeanGlyph /></span><h3 className="h3">Pro</h3></div>
              <div className="price-row"><span className="bignum">$5</span><span className="price-unit">one-off</span></div>
              <div className="notebox">One-time payment. No subscription.</div>
              <List items={["Everything in Free", "Pro features coming soon"]} />
              <span className="btn btn--lg" aria-disabled="true">Coming soon</span>
            </div>
          </div>
        </div>
        <p className="small center" style={{ marginTop: 24 }} data-reveal>Pro is not on sale yet. You will not be charged for anything today.</p>
      </div>
    </section>
  )
}

export function Faq() {
  return (
    <section className="section" id="faq">
      <div className="faq-wrap">
        <div className="stack center" style={{ textAlign: "center" }}>
          <h2 className="h2" data-reveal>Good to know.</h2>
          <p className="lead" data-reveal data-delay="80">The short answers about the taskbar, your tray and your PC.</p>
        </div>
        <div style={{ position: "relative" }}>
          <Bean style={{ width: 88, top: 28, right: 24, transform: "rotate(12deg)" }} />
          <div className="faq">
            {FAQ.map((f, i) => (
              <details className="faq__item text-cut" key={f.q} data-reveal data-delay={i * 60}>
                <summary className="faq__q"><Plus />{f.q}</summary>
                <p className="faq__a">{f.a}</p>
              </details>
            ))}
          </div>
        </div>
      </div>
    </section>
  )
}

export function FinalCta() {
  return (
    <section className="section">
      <div className="wrap">
        <div className="closer center">
          <div className="stack center">
            <h2 className="h2" data-reveal>Give Windows a menu bar.</h2>
            <p className="lead" style={{ maxWidth: 520 }} data-reveal data-delay="80">
              One download. Your tray, your notifications and your controls, all at the top.
            </p>
            <div data-reveal data-delay="160"><Download white /></div>
          </div>
          <Bean style={{ width: 150, transform: "rotate(-8deg)" }} />
        </div>
      </div>
    </section>
  )
}
