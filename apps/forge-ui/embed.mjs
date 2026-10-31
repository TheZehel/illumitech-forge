import {cp, mkdir, readdir, rm, readFile, writeFile} from 'node:fs/promises';
const destination = new URL('../../crates/forge-cli/ui-dist/', import.meta.url);
await mkdir(destination, {recursive:true});
// This directory contains only generated assets; source and operator state are untouched.
for (const name of await readdir(destination)) await rm(new URL(name,destination),{recursive:true});
await cp(new URL('./dist/',import.meta.url),destination,{recursive:true});

let licenses = 'Illumitech Forge UI — MIT. Runtime dependency notices follow.\n';
for (const name of ['react', 'react-dom', 'scheduler']) {
  const license = await readFile(new URL(`./node_modules/${name}/LICENSE`, import.meta.url), 'utf8');
  licenses += `\n=== ${name} (MIT) ===\n${license}\n`;
}
await writeFile(new URL('THIRD_PARTY_LICENSES.txt', destination), licenses);
