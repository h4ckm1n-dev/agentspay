import { SITE } from "@/lib/seo";

interface JsonLdProps {
  readonly id: string;
  readonly data: Record<string, unknown>;
}

function JsonLd({ id, data }: JsonLdProps) {
  return (
    <script id={id} type="application/ld+json" suppressHydrationWarning>
      {JSON.stringify(data)}
    </script>
  );
}

export function StructuredData() {
  const softwareApplication = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: SITE.name,
    applicationCategory: "DeveloperApplication",
    applicationSubCategory: "MCP server",
    operatingSystem: "macOS, Linux, Windows",
    description: SITE.description,
    url: SITE.url,
    downloadUrl: `${SITE.github}/releases`,
    softwareVersion: "0.3",
    license: "https://opensource.org/license/mit",
    offers: {
      "@type": "Offer",
      price: "0",
      priceCurrency: "USD",
    },
    author: {
      "@type": "Person",
      name: SITE.repoOwnerName,
      url: SITE.github,
    },
    sameAs: [SITE.github],
    keywords: SITE.keywords.join(", "),
  };

  const organization = {
    "@context": "https://schema.org",
    "@type": "Organization",
    name: SITE.name,
    url: SITE.url,
    logo: `${SITE.url}/agentspay-mark.svg`,
    description: SITE.shortDescription,
    sameAs: [SITE.github],
  };

  const website = {
    "@context": "https://schema.org",
    "@type": "WebSite",
    name: SITE.name,
    url: SITE.url,
    description: SITE.shortDescription,
    publisher: { "@type": "Organization", name: SITE.name },
    inLanguage: "en",
  };

  return (
    <>
      <JsonLd id="ld-software" data={softwareApplication} />
      <JsonLd id="ld-organization" data={organization} />
      <JsonLd id="ld-website" data={website} />
    </>
  );
}
