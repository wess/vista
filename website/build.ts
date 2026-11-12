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
  <link rel="icon" href="${current ? "/vista/icon.png" : "icon.png"}">
  <link rel="stylesheet" href="${current ? "/vista/style.css" : "style.css"}">
  <title>${escape(title)} · Vista</title>
</head>
<body class="${current ? "docs-page" : "home-page"}">
  ${current ? `<header class="topbar">
    <a class="brand" href="/vista/"><img src="/vista/icon.png" alt=""><span>Vista</span></a>
    <nav aria-label="Primary"><a href="/vista/docs/install/">Documentation</a><a href="${repository}">GitHub</a></nav>
  </header>` : ""}
  ${current ? `<div class="docs"><aside><label for="search">Search documentation</label><input id="search" type="search" placeholder="Search"><div class="searchstatus" role="status" aria-live="polite"></div><nav class="docnav" aria-label="Documentation">${nav(current)}</nav></aside>` : ""}
  <main id="content"${current ? ' class="article"' : ""}>${body}</main>
  ${current ? "</div>" : ""}
  <footer><span>Vista is free and open source.</span><a href="${repository}">Apache-2.0 on GitHub</a></footer>
  <script src="${current ? "/vista/site.js" : "site.js"}" defer></script>
</body>
</html>`;

const home = `<section class="retro-shell comp-frame" aria-label="VISTA 128 desktop">
  <div class="r-page-shell case-surface" aria-hidden="true"></div>
  <header class="os-menubar-group">
    <div class="r-menu-bar os-menubar" aria-hidden="true"></div>
    <a class="r-brand retro-brand" href="/vista/"><span class="brand-stripes" aria-hidden="true"><i></i><i></i><i></i></span><strong>VISTA</strong></a>
    <nav class="r-menu-items os-menu" aria-label="Primary"><a href="/vista/">VISTA</a><a href="/vista/docs/install/">FILE</a><a href="/vista/docs/machines/">MACHINE</a><a href="/vista/docs/menubar/">WINDOW</a><a href="/vista/docs/troubleshooting/">HELP</a></nav>
    <span class="r-version version">0.1.0</span>
  </header>
  <div class="r-desktop os-desktop" aria-hidden="true"></div>
  <nav class="r-desktop-icons desktop-icons" aria-label="Desktop shortcuts">
    <a href="/vista/"><span class="pixel-icon computer-icon" aria-hidden="true"></span><b>VISTA</b></a>
    <a href="/vista/docs/systems/"><span class="pixel-icon disk-icon" aria-hidden="true"></span><b>CATALOG</b></a>
    <a href="/vista/docs/install/"><span class="pixel-icon book-icon" aria-hidden="true"></span><b>DOCS</b></a>
    <a href="/vista/docs/settings/"><span class="pixel-icon gear-icon" aria-hidden="true"></span><b>SETTINGS</b></a>
  </nav>
  <section class="window-group" aria-labelledby="catalog-title">
    <div class="r-catalog-window retro-window catalog-window" aria-hidden="true"><div class="window-bar"><span class="window-glyph"></span></div></div>
    <h2 id="catalog-title" class="r-catalog-title">CATALOG</h2>
    <ul class="r-catalog-list">
      <li><span>■ macOS</span><b>[STABLE]</b></li><li><span>■ Ubuntu</span><b>[STABLE]</b></li><li><span>■ Fedora</span><b>[STABLE]</b></li><li><span>■ Debian</span><b>[STABLE]</b></li><li><span>□ BSD</span><b>[STABLE]</b></li><li><span>□ Windows 11 Arm</span><b>[EXPERIMENTAL]</b></li>
    </ul>
  </section>
  <section class="window-group" aria-labelledby="monitor-title">
    <div class="r-monitor-window retro-window monitor-window" aria-hidden="true"><div class="window-bar"><span class="monitor-bars"><i></i><i></i><i></i></span></div></div>
    <h2 id="monitor-title" class="r-monitor-title">SYSTEM MONITOR</h2>
    <ul class="r-monitor-list"><li>■ 0 TELEMETRY</li><li>■ 1 PROCESS PER GUEST</li><li>■ APFS SNAPSHOTS</li></ul>
  </section>
  <section class="window-group" aria-labelledby="terminal-title">
    <div class="r-terminal-window retro-window terminal-window" aria-hidden="true"><div class="window-bar"><span class="prompt-icon">&gt;_</span></div></div>
    <h1 id="terminal-title" class="r-terminal-title">VISTA 128</h1>
    <div class="r-terminal-controls window-controls" aria-hidden="true"><i>−</i><i>□</i><i>×</i></div>
  </section>
  <div class="r-terminal-screen terminal-screen" data-terminal-output aria-live="polite"></div>
  <div class="r-vista-masthead screen-masthead"><span class="masthead-stripes" aria-hidden="true">///</span> VISTA 128</div>
  <p class="r-vista-tagline screen-tagline">NATIVE VIRTUAL MACHINES FOR APPLE SILICON</p>
  <div class="r-ready-command terminal-copy"><span>READY.</span><code>&gt; brew install --cask wess/packages/vista</code></div>
  <p class="r-command-help terminal-help">TYPE A COMMAND: CATALOG, INSTALL, DOCS,<br>SECURITY, OR CLEAR.</p>
  <form class="r-active-prompt terminal-form" data-terminal-form><label class="sr-only" for="retro-command">Terminal command</label><span>&gt;</span><input id="retro-command" data-terminal-input autocomplete="off" spellcheck="false" aria-describedby="terminal-instructions"></form>
  <p id="terminal-instructions" class="sr-only">Type help, catalog, install, docs, security, or clear, then press Return.</p>
  <a class="r-download-key download-key" href="${repository}/releases/latest">DOWNLOAD 0.1.0</a>
  <div class="r-ready-status ready-status"><i aria-hidden="true"></i><span>READY.</span></div>
  <nav class="manual-strip-group" aria-label="Manual shortcuts">
    <div class="r-manual-strip manual-strip" aria-hidden="true"></div>
    <strong class="r-manual-title">VISTA MANUAL</strong>
    <a class="r-manual-docs" href="/vista/docs/install/"><span class="manual-symbol book-symbol" aria-hidden="true"></span><b>DOCUMENTATION</b><small>GUIDES, CONFIGURATION, AND USAGE.</small></a>
    <a class="r-manual-security" href="/vista/docs/security/"><span class="manual-symbol shield-symbol" aria-hidden="true"></span><b>SECURITY</b><small>ISOLATION, DATA, AND TRUST.</small></a>
    <a class="r-manual-catalog" href="/vista/docs/systems/"><span class="manual-symbol stack-symbol" aria-hidden="true"></span><b>MACHINE CATALOG</b><small>SUPPORTED GUESTS AND STATUS.</small></a>
  </nav>
</section>`;

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
await mkdir(join(output, "fonts"), { recursive: true });
for (const font of await readdir(join(import.meta.dir, "fonts"))) {
  await Bun.write(join(output, "fonts", font), Bun.file(join(import.meta.dir, "fonts", font)));
}
await Bun.write(join(output, ".nojekyll"), "");

console.log(`Built Vista site with ${pages.length} documentation pages.`);
