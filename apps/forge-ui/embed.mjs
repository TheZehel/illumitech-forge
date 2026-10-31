import {cp, mkdir, readdir, rm, readFile, writeFile} from 'node:fs/promises';
const destination = new URL('../../crates/forge-cli/ui-dist/', import.meta.url);
await mkdir(destination, {recursive:true});
// This directory contains only generated assets; source and operator state are untouched.
for (const name of await readdir(destination)) await rm(new URL(name,destination),{recursive:true});
await cp(new URL('./dist/',import.meta.url),destination,{recursive:true});
let licenses = 'Illumitech Forge UI — MIT. Runtime dependency notices follow.\n';
const lock = JSON.parse(await readFile(new URL('./package-lock.json', import.meta.url), 'utf8'));
for (const [location, pkg] of Object.entries(lock.packages)) {
  if (!location || pkg.dev) continue;
  const name = location.split('node_modules/').at(-1);
  const source = new URL(`./${location}/`, import.meta.url);
  const notices = (await readdir(source)).filter(name => /^(LICENSE|COPYING|NOTICE|UNLICENSE)/i.test(name));
  if (!pkg.license || !notices.length) throw new Error(`Missing runtime license notice: ${name}`);
  licenses += `\n=== ${name}@${pkg.version} (${pkg.license}) ===\n`;
  for (const file of notices) licenses += await readFile(new URL(file, source), 'utf8') + '\n';
}
await writeFile(new URL('THIRD_PARTY_LICENSES.txt', destination), licenses);
