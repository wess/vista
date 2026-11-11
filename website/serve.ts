import { extname, join, normalize } from "node:path";

const root = join(import.meta.dir, "..", "site");
const types: Record<string, string> = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".txt": "text/plain; charset=utf-8",
  ".xml": "application/xml; charset=utf-8",
};

const server = Bun.serve({
  port: Number(Bun.env.PORT ?? 4173),
  async fetch(request) {
    const url = new URL(request.url);
    let path = decodeURIComponent(url.pathname).replace(/^\/vista\/?/, "");
    if (!path || path.endsWith("/")) path += "index.html";
    const safe = normalize(path).replace(/^(\.\.\/)+/, "");
    const file = Bun.file(join(root, safe));
    if (!(await file.exists())) return new Response("Not found", { status: 404 });
    return new Response(file, { headers: { "content-type": types[extname(safe)] ?? "application/octet-stream" } });
  },
});

console.log(`Vista site: http://127.0.0.1:${server.port}/vista/`);
