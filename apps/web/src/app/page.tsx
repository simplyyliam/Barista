import { Footer } from "@/components/landing/footer"
import { Nav } from "@/components/landing/nav"
// Hidden for v1 ("See it at work"). Bring it back with the import and the <TourSection /> line below.
// import { TourSection } from "@/components/landing/explorer"
import { RevealController } from "@/components/landing/reveal"
import { SmoothScroll } from "@/components/landing/smooth-scroll"
import {
  Faq, Features, FinalCta, Hero, Light, Sdk, Statement, Twin,
  // Pricing, // Hidden for the beta: no Pro yet. See docs/release/plan/hidden-on-website.md.
} from "@/components/landing/sections"

export default function Home() {
  return (
    <div className="sl">
      <Nav />
      <main>
        <Hero />
        <Statement />
        <Features />
        {/* <TourSection /> */}
        <Sdk />
        <Light />
        <Twin />
        {/* <Pricing /> */}
        <Faq />
        <FinalCta />
      </main>
      <Footer />
      <RevealController />
      <SmoothScroll />
    </div>
  )
}
