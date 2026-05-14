import { Hero } from "@/components/sections/Hero";
import { Install } from "@/components/sections/Install";
import { LiveDemo } from "@/components/sections/LiveDemo";
import { Footer } from "@/components/sections/Footer";

export default function Home() {
  return (
    <main>
      <Hero />
      <Install />
      <LiveDemo />
      <Footer />
    </main>
  );
}
