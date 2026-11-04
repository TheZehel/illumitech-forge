import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = dirname(fileURLToPath(import.meta.url));
const {published} = JSON.parse(await readFile(join(root,'../../release/current.json'),'utf8'));
const currentVersion = new RegExp(published.replaceAll('.', '\\.')); 
for (const lang of ['pt','en']) test(`Static ${lang} preview has real install instructions and no local capability`,async()=>{
 const html=await readFile(join(root,'dist',lang,'index.html'),'utf8');
 assert.match(html,currentVersion);assert.doesNotMatch(html,/1\.1\.1/);assert.match(html,/<html lang=/);assert.match(html,/<main id="main">/);
 assert.doesNotMatch(html,/localhost|127\.0\.0\.1|X-Forge-Token|\/api\/|\/Users\/|\/Volumes\/|<script/);
 for(const match of html.matchAll(/(?:href|src)="([^"#]+)"/g)){const href=match[1];if(href.startsWith('https:'))continue;const file=resolve(root,'dist',lang,href);const stat=await import('node:fs/promises').then(fs=>fs.stat(file));assert.ok(stat.isFile()||stat.isDirectory(),href);}
});
test('No source maps, sessions or private source enter the static output',async()=>{const paths=await readdir(join(root,'dist'),{recursive:true});assert.ok(!paths.some(p=>/\.map$|\.sqlite|candidate\.json|AGENTS\.md|Cargo\.toml|auth\.json/.test(p)));});
