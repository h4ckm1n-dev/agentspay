import type { Metadata, Viewport } from "next";
import { SiteNav } from "@/components/layout/SiteNav";
import { SITE } from "@/lib/seo";
import { StructuredData } from "@/components/seo/StructuredData";
import "./globals.css";

export const metadata: Metadata = {
  metadataBase: new URL(SITE.url),
  title: {
    default: `${SITE.name} — ${SITE.tagline}`,
    template: `%s — ${SITE.name}`,
  },
  description: SITE.shortDescription,
  applicationName: SITE.name,
  keywords: [...SITE.keywords],
  authors: [{ name: SITE.repoOwnerName, url: SITE.github }],
  creator: SITE.repoOwnerName,
  publisher: SITE.name,
  category: "technology",
  alternates: {
    canonical: "/",
  },
  openGraph: {
    type: "website",
    locale: SITE.locale,
    url: SITE.url,
    siteName: SITE.name,
    title: `${SITE.name} — ${SITE.tagline}`,
    description: SITE.shortDescription,
    images: [
      {
        url: SITE.ogImagePath,
        width: 1200,
        height: 630,
        alt: `${SITE.name} — ${SITE.tagline}`,
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: `${SITE.name} — ${SITE.tagline}`,
    description: SITE.shortDescription,
    images: [SITE.ogImagePath],
    creator: SITE.twitter,
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-image-preview": "large",
      "max-snippet": -1,
      "max-video-preview": -1,
    },
  },
  icons: {
    icon: [{ url: "/favicon.svg", type: "image/svg+xml" }],
    shortcut: "/favicon.svg",
    apple: "/favicon.svg",
  },
};

export const viewport: Viewport = {
  themeColor: SITE.themeColor,
  colorScheme: "dark",
  width: "device-width",
  initialScale: 1,
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>
        <StructuredData />
        <SiteNav />
        {children}
      </body>
    </html>
  );
}
