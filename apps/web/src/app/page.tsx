import { Footer } from "@/components/landing/footer"
import { Nav } from "@/components/landing/nav"
import { RevealController } from "@/components/landing/reveal"
import {
  Faq, Features, FinalCta, Hero, Light, Pricing, Sdk, Statement, Twin,
} from "@/components/landing/sections"

export default function Home() {
  return (
    <div className="sl">
      <Nav />
      <main>
        <Hero />
        <Statement />
        <Features />
        <Sdk />
        <Light />
        <Twin />
        <Pricing />
        <Faq />
        <FinalCta />
      </main>
      <Footer />
      <RevealController />
    </div>
  )
}
