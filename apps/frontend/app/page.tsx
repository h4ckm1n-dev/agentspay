import { Hero } from "@/components/sections/Hero";
import { ProofStrip } from "@/components/sections/ProofStrip";
import { HowItWorks } from "@/components/sections/HowItWorks";
import { Install } from "@/components/sections/Install";
import { Why } from "@/components/sections/Why";
import { Footer } from "@/components/sections/Footer";

export default function Home() {
  return (
    <main>
      <Hero />
      <ProofStrip />
      <HowItWorks />
      <Install />
      <Why />
      <Footer />
    </main>
  );
}
