import { mkdir, readdir, rm } from "node:fs/promises";
import { basename, join } from "node:path";

const root = join(import.meta.dir, "..");
const output = join(root, "site");
const siteurl = "https://wess.io/vista/";
const repository = "https://github.com/wess/vista";

type Page = {
  source: string;
  slug: string;
  title: string;
  description: string;
  markdown: string;
};

const escape = (value: string): string =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");

const plain = (value: string): string =>
  value
    .replace(/!\[[^\]]*\]\([^)]*\)/g, "")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/[`*_>#]/g, "")
    .replace(/\s+/g, " ")
    .trim();

const heading = (markdown: string): string =>
  markdown.match(/^#\s+(.+)$/m)?.[1]?.trim() ?? "Vista";

const description = (markdown: string): string => {
  const blocks = markdown.split(/\n\s*\n/).slice(1);
  const paragraph = blocks.find((block) => !/^(?:#|\||-|```|!\[)/.test(block.trim()));
  return plain(paragraph ?? "Virtual machines for Apple silicon.").slice(0, 180);
};

const slugify = (value: string): string =>
  plain(value)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");

const markup = (markdown: string): string => {
  const ids = new Map<string, number>();
  return Bun.markdown
    .html(markdown)
    .replace(/<h([1-6])>(.*?)<\/h\1>/g, (_, level, content) => {
      const base = slugify(content) || "section";
      const count = ids.get(base) ?? 0;
      ids.set(base, count + 1);
      const id = count === 0 ? base : `${base}-${count + 1}`;
      return `<h${level} id="${id}"><a class="anchor" href="#${id}" aria-label="Link to ${escape(plain(content))}">#</a>${content}</h${level}>`;
    })
    .replace(/href="(?:\.\.\/)?docs\/([a-z]+)\.md(#[^"]+)?"/g, 'href="/vista/docs/$1/$2"')
    .replace(/href="([a-z]+)\.md(#[^"]+)?"/g, 'href="/vista/docs/$1/$2"')
    .replace(/href="(?:\.\.\/)?README\.md"/g, 'href="/vista/"')
    .replace(/src="assets\/icon\.png"/g, 'src="/vista/icon.png"')
    .replace(/href="(\/vista\/docs\/[a-z]+\/)undefined"/g, 'href="$1"');
};

const docfiles = (await readdir(join(root, "docs")))
  .filter((name) => name.endsWith(".md") && name !== "readme.md")
  .sort();

const pages: Page[] = await Promise.all(
  docfiles.map(async (source) => {
    const markdown = await Bun.file(join(root, "docs", source)).text();
    return {
      source,
      slug: basename(source, ".md"),
      title: heading(markdown),
      description: description(markdown),
      markdown,
    };
  }),
);

const nav = (current?: string): string =>
  pages
    .map(
      (page) =>
        `<a href="/vista/docs/${page.slug}/"${page.slug === current ? ' aria-current="page"' : ""}>${escape(page.title)}</a>`,
    )
    .join("\n");

const shell = (title: string, summary: string, body: string, path: string, current?: string): string => `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="description" content="${escape(summary)}">
  <meta name="theme-color" content="#090b10">
  <meta property="og:title" content="${escape(title)}">
  <meta property="og:description" content="${escape(summary)}">
  <meta property="og:type" content="website">
  <meta property="og:url" content="${siteurl}${path}">
  <meta property="og:image" content="${siteurl}icon.png">
  <link rel="canonical" href="${siteurl}${path}">
  <link rel="icon" href="/vista/icon.png">
  <link rel="stylesheet" href="/vista/style.css">
  <title>${escape(title)} · Vista</title>
</head>
<body>
  <header class="topbar">
    <a class="brand" href="/vista/"><img src="/vista/icon.png" alt=""><span>Vista</span></a>
    <nav aria-label="Primary"><a href="/vista/docs/install/">Documentation</a><a href="${repository}">GitHub</a></nav>
  </header>
  ${current ? `<div class="docs"><aside><label for="search">Search documentation</label><input id="search" type="search" placeholder="Search"><div class="searchstatus" role="status" aria-live="polite"></div><nav class="docnav" aria-label="Documentation">${nav(current)}</nav></aside>` : ""}
  <main id="content"${current ? ' class="article"' : ""}>${body}</main>
  ${current ? "</div>" : ""}
  <footer><span>Vista is free and open source.</span><a href="${repository}">Apache-2.0 on GitHub</a></footer>
  <script src="/vista/site.js" defer></script>
</body>
</html>`;

const home = `<section class="hero">
  <div class="eyebrow">Virtualization, native to the Mac</div>
  <h1>Your operating systems.<br><em>In their own windows.</em></h1>
  <p>Vista finds, verifies, builds, and runs virtual machines on Apple silicon using macOS’s own Virtualization framework.</p>
  <div class="actions"><a class="button primary" href="${repository}/releases/latest">Download Vista</a><a class="button" href="/vista/docs/install/">Read the documentation</a></div>
  <div class="requirements">Apple silicon · macOS 15 or later · no account, licence key, or telemetry</div>
</section>
<section class="systems" aria-labelledby="systems-title">
  <div><span class="kicker">A broad catalog</span><h2 id="systems-title">From macOS to the BSDs.</h2></div>
  <p>Install current macOS, Ubuntu, Fedora, Debian, Kali, Rocky, AlmaLinux, openSUSE, Pop!_OS, NixOS, Alpine, FreeBSD, OpenBSD, NetBSD, and experimental Windows 11 for Arm.</p>
</section>
<section class="features" aria-label="Highlights">
  <article><span>01</span><h2>One-click machines</h2><p>Vista resolves the current installer from its publisher, verifies the published checksum, and keeps the image for next time.</p></article>
  <article><span>02</span><h2>Snapshots without the wait</h2><p>APFS copy-on-write snapshots and clones start nearly free, while suspend preserves a running machine’s memory.</p></article>
  <article><span>03</span><h2>Built for automation</h2><p>The included MCP server exposes lifecycle, configuration, screen, input, snapshot, command, and file tools.</p></article>
  <article><span>04</span><h2>Mac-native boundaries</h2><p>Each guest runs in its own process. Virtualization, networking, signing, and permissions stay on the platform’s native path.</p></article>
</section>
<section class="guide"><span class="kicker">Start here</span><h2>Install the app, then build a machine.</h2><div><a href="/vista/docs/install/"><strong>Installing Vista</strong><small>Requirements, first launch, and storage</small></a><a href="/vista/docs/machines/"><strong>Creating machines</strong><small>Catalog systems, custom images, and jobs</small></a><a href="/vista/docs/agents/"><strong>Connect an agent</strong><small>Control machines through the MCP server</small></a></div></section>`;

await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });

await Bun.write(
  join(output, "index.html"),
  shell("Virtual machines for Apple silicon", "Native virtual machines for Apple silicon, free and open source.", home, ""),
);

for (const page of pages) {
  const target = join(output, "docs", page.slug);
  await mkdir(target, { recursive: true });
  const body = `<div class="breadcrumb"><a href="/vista/">Vista</a><span>/</span>Documentation</div>${markup(page.markdown)}<p class="source"><a href="${repository}/blob/main/docs/${page.source}">Edit this page on GitHub</a></p>`;
  await Bun.write(join(target, "index.html"), shell(page.title, page.description, body, `docs/${page.slug}/`, page.slug));
}

const docsText = pages
  .map((page) => `## ${page.title}\n\nSource: ${siteurl}docs/${page.slug}/\n\n${page.markdown.replace(/^#\s+.+\n/, "").trim()}`)
  .join("\n\n---\n\n");
const llms = `# Vista\n\n> Native virtual machines for Apple silicon, built on Apple's Virtualization framework.\n\nRepository: ${repository}\nDocumentation: ${siteurl}\nLatest release: ${repository}/releases/latest\n\n${docsText}\n`;
await Bun.write(join(output, "llms.txt"), llms);
await Bun.write(join(output, "llm.txt"), llms);
await Bun.write(join(output, "search.json"), JSON.stringify(pages.map((page) => ({ title: page.title, description: page.description, path: `/vista/docs/${page.slug}/`, text: plain(page.markdown) }))));
await Bun.write(join(output, "robots.txt"), `User-agent: *\nAllow: /\nSitemap: ${siteurl}sitemap.xml\n`);
await Bun.write(join(output, "sitemap.xml"), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n  <url><loc>${siteurl}</loc></url>${pages.map((page) => `\n  <url><loc>${siteurl}docs/${page.slug}/</loc></url>`).join("")}\n</urlset>\n`);
await Bun.write(join(output, "style.css"), Bun.file(join(import.meta.dir, "style.css")));
await Bun.write(join(output, "site.js"), Bun.file(join(import.meta.dir, "site.js")));
await Bun.write(join(output, "icon.png"), Bun.file(join(root, "assets", "icon.png")));
await Bun.write(join(output, ".nojekyll"), "");

console.log(`Built Vista site with ${pages.length} documentation pages.`);
