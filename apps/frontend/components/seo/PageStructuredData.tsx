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

interface BreadcrumbProps {
  readonly trail: ReadonlyArray<{
    readonly name: string;
    readonly url: string;
  }>;
}

export function BreadcrumbStructuredData({ trail }: BreadcrumbProps) {
  const data = {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: trail.map((entry, index) => ({
      "@type": "ListItem",
      position: index + 1,
      name: entry.name,
      item: entry.url.startsWith("http")
        ? entry.url
        : `${SITE.url}${entry.url}`,
    })),
  };
  return <JsonLd id="ld-breadcrumb" data={data} />;
}

interface TechArticleProps {
  readonly headline: string;
  readonly description: string;
  readonly path: string;
  readonly sections: ReadonlyArray<string>;
}

export function TechArticleStructuredData({
  headline,
  description,
  path,
  sections,
}: TechArticleProps) {
  const url = `${SITE.url}${path}`;
  const data = {
    "@context": "https://schema.org",
    "@type": "TechArticle",
    headline,
    description,
    url,
    mainEntityOfPage: { "@type": "WebPage", "@id": url },
    inLanguage: "en",
    isPartOf: { "@type": "WebSite", name: SITE.name, url: SITE.url },
    author: {
      "@type": "Person",
      name: SITE.repoOwnerName,
      url: SITE.github,
    },
    publisher: {
      "@type": "Organization",
      name: SITE.name,
      url: SITE.url,
      logo: { "@type": "ImageObject", url: `${SITE.url}/agentspay-mark.svg` },
    },
    articleSection: [...sections],
    proficiencyLevel: "Beginner",
    dependencies:
      "macOS, Linux, or Windows · Rust toolchain or pre-built binary · An MCP host (Claude Code, Cursor, Cline, Zed)",
  };
  return <JsonLd id="ld-techarticle" data={data} />;
}
