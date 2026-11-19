import { WindowsGlyph } from "./icons"
import { Wordmark } from "./wordmark"
import { DOWNLOAD_URL, NPM_URL } from "./site"

export function Footer() {
  return (
    <footer>
      <div className="footer">
        <div className="footer__brand" data-reveal>
          <span className="brand" style={{ fontSize: 20 }}>Barista</span>
          <p className="footer__tag">Your apps, served at the top.</p>
          <a href={DOWNLOAD_URL} className="btn btn--sm btn--primary">
            <WindowsGlyph />
            Download for Windows
          </a>
          <p className="copy">© {new Date().getFullYear()} Barista. All rights reserved.</p>
          <p className="copy">Built with 💙 by Liam</p>
        </div>
        <div data-reveal data-delay="80">
          <h4>Product</h4>
          <ul>
            <li><a href="#features">Features</a></li>
            <li><a href="#pricing">Pricing</a></li>
            <li><a href="#faq">FAQ</a></li>
          </ul>
        </div>
        <div data-reveal data-delay="160">
          <h4>Developers</h4>
          <ul>
            <li><a href="/docs/">Docs</a></li>
            <li><a href="/docs/protocol/">Protocol</a></li>
            <li><a href={NPM_URL}>SDK on npm</a></li>
          </ul>
        </div>
        <div data-reveal data-delay="240">
          <h4>More</h4>
          <ul>
            <li><a href={DOWNLOAD_URL}>Releases</a></li>
            <li><a href="https://github.com/simplyyliam/barista-releases">GitHub</a></li>
          </ul>
        </div>
      </div>
      <Wordmark />
    </footer>
  )
}
