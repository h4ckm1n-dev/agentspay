import { Hero } from "@/components/sections/Hero";
import { Install } from "@/components/sections/Install";
import { LiveDemo } from "@/components/sections/LiveDemo";
import { HowItWorks } from "@/components/sections/HowItWorks";
import { Why } from "@/components/sections/Why";
import { Footer } from "@/components/sections/Footer";

export default function Home() {
  return (
    <main>
      <Hero />
      <Install />
      <LiveDemo />
      <HowItWorks />
      <Why />
      <Footer />
    </main>
  );
}
