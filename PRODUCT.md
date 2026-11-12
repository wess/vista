# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Primary users are Apple-silicon Mac owners who want to create and operate local virtual machines without learning a separate hypervisor stack. A second audience is developers connecting MCP-capable tools to those machines. *(Inferred from the repository and current documentation.)*

## Product Purpose

Vista finds current operating-system installers at their publishers, verifies them when a checksum is available, builds virtual machines, and runs each guest in its own process through Apple's Virtualization framework. The website must explain the product quickly, provide a trustworthy install path, and make the complete technical documentation easy to navigate.

## Positioning

Vista combines a native Mac application, a maintained catalog of current operating systems, APFS-native snapshots and clones, and a built-in MCP server. It has no accounts, licence keys, telemetry, or splash screens.

## Operating Context

Visitors evaluate Vista on a Mac, install it through Homebrew or a signed disk image, create machines from the catalog or their own Arm64 media, and use the documentation while configuring hardware, sharing, snapshots, guest tools, or agent access.

## Capabilities and Constraints

- Apple silicon and macOS 15 or later are required.
- Windows 11 for Arm support is experimental and must be described honestly.
- The static Bun-generated site is published under `/vista/` on GitHub Pages.
- All existing documentation, canonical metadata, search, `llm.txt`, `llms.txt`, sitemap, and source links must remain functional.
- Product claims must come from repository documentation; do not invent customers, benchmarks, or compatibility claims.

## Brand Commitments

- The product name is Vista and the existing app icon remains an identifying asset.
- The voice is direct, technical, modest, and occasionally playful.
- The replacement web identity is retro-computing rather than contemporary SaaS. The user named `ainz.sh`, `wess.tools`, `wess.email`, and `wess.io` as binding craft references and invited a Commodore 128-style computer interface.
- Because the session's decision tool was unavailable, this build treats the C128-style interface as the working home-page centerpiece rather than a decorative secondary element. *(Explicitly inferred from the user's strongest suggestion; open to revision.)*

## Evidence on Hand

- Product overview and install instructions: `README.md`
- Seventeen maintained guides: `docs/*.md`
- App icon: `assets/icon.png`
- Security and architectural boundaries: `docs/security.md`, `docs/architecture.md`
- No testimonials, customer logos, performance benchmarks, or usage statistics are available and none may be fabricated.

## Product Principles

- Native Mac behavior before cross-platform convention.
- Show operational truth instead of broad marketing claims.
- Keep local ownership and explicit trust boundaries visible.
- Make advanced automation approachable without hiding risk or limitations.
- Prefer a small, fast, dependency-light website.

## Accessibility & Inclusion

The site must remain keyboard operable, readable under reduced motion, responsive on mobile, and understandable without relying on color or animation alone.
