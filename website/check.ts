import { readdir } from "node:fs/promises";
import { extname, join, relative } from "node:path";

const output = join(import.meta.dir, "..", "site");
const failures: string[] = [];
const walk = async (dir: string): Promise<string[]> =>
  (await Promise.all((await readdir(dir, { withFileTypes: true })).map((entry) => entry.isDirectory() ? walk(join(dir, entry.name)) : [join(dir, entry.name)]))).flat();
const files = await walk(output);
const names = new Set(files.map((file) => relative(output, file)));
const html = files.filter((file) => extname(file) === ".html");

for (const file of html) {
  const path = relative(output, file);
  const source = await Bun.file(file).text();
  for (const required of ['<html lang="en">', '<main id="content"', '<meta name="description"', '<link rel="canonical"']) {
    if (!source.includes(required)) failures.push(`${path}: missing ${required}`);
  }
  const imageWithoutAlt = new RegExp("<" + "img(?![^>]*\\balt=)[^>]*>", "g");
  if (imageWithoutAlt.test(source)) failures.push(`${path}: image without alt`);
  for (const match of source.matchAll(/(?:href|src)="(\/vista\/[^"#?]*)/g)) {
    let target = match[1].slice("/vista/".length);
    if (!target || target.endsWith("/")) target += "index.html";
    if (!names.has(target)) failures.push(`${path}: missing ${match[1]} -> ${target}`);
  }
}

const docs = html.filter((file) => relative(output, file).startsWith("docs/"));
if (docs.length < 16) failures.push(`expected at least 16 documentation pages, found ${docs.length}`);
for (const name of ["llm.txt", "llms.txt", "robots.txt", "sitemap.xml", ".nojekyll"]) {
  if (!names.has(name)) failures.push(`missing ${name}`);
}
const llms = await Bun.file(join(output, "llms.txt")).text();
for (const phrase of ["Installing Vista", "MCP tool reference", "Vista Tools", "Architecture"]) {
  if (!llms.includes(phrase)) failures.push(`llms.txt is missing ${phrase}`);
}

if (failures.length) {
  console.error(failures.join("\n"));
  process.exit(1);
}
console.log(`Checked ${html.length} HTML pages and ${files.length} generated files.`);
