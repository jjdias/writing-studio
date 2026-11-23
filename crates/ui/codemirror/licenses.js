// Gathers the license of every package in the bundle into LICENSES.txt, which
// the Package workflow puts in each package's licenses folder.
const fs = require("fs");
const path = require("path");

const lock = JSON.parse(fs.readFileSync("package-lock.json", "utf8"));
// Only what esbuild put in the bundle, from the metafile it wrote.
const meta = JSON.parse(fs.readFileSync("node_modules/.bundle-meta.json", "utf8"));
const inputs = Object.keys(meta.inputs).map((file) => file.replace(/\\/g, "/"));
const bundled = Object.entries(lock.packages)
  .filter(([where, info]) => where.startsWith("node_modules/") && !info.dev && inputs.some((file) => file.startsWith(where + "/")))
  .map(([where, info]) => ({ where, name: where.slice("node_modules/".length), version: info.version, license: info.license }))
  .sort((a, b) => a.name.localeCompare(b.name));

let out = "Writing Studio's Markdown editor is built from CodeMirror 6 and the packages below, bundled\ninto one file. Each is under the license that follows its name.\n";
for (const pkg of bundled) {
  const dir = path.join(pkg.where);
  const file = fs.readdirSync(dir).find((name) => /^(licen[cs]e|copying)(\.|$)/i.test(name));
  if (!file) throw new Error(`${pkg.name} has no license file`);
  out += `\n${"=".repeat(72)}\n${pkg.name} ${pkg.version} (${pkg.license})\n${"=".repeat(72)}\n\n`;
  out += fs.readFileSync(path.join(dir, file), "utf8").trim() + "\n";
}
fs.writeFileSync("LICENSES.txt", out);
console.log(`LICENSES.txt: ${bundled.length} packages`);
