import { Hero } from "@/components/sections/Hero";
import { ProofStrip } from "@/components/sections/ProofStrip";
import { Install } from "@/components/sections/Install";
import { Why } from "@/components/sections/Why";
import { Footer } from "@/components/sections/Footer";

export default function Home() {
  return (
    <main>
      <Hero />
      <ProofStrip />
      <Install />
      <Why />
      <Footer />
    </main>
  );
}
