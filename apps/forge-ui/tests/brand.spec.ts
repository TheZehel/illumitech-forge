import { test, expect } from '@playwright/test';
import { createServer, type Server } from 'node:http';
import { readFile, stat, mkdtemp, mkdir, rm } from 'node:fs/promises';
import { resolve, join, extname } from 'node:path';
import { tmpdir } from 'node:os';
import { spawn, type ChildProcess } from 'node:child_process';

let server: Server;
let base = '';
let child: ChildProcess;
let directory = '';
let workbench = '';
test.beforeAll(async () => {
    const site = resolve('../forge-site/dist');
    server = createServer((req, res) => {
        void (async () => {
            let file = join(site, decodeURIComponent(req.url?.split('?')[0] ?? '/'));
            if (!file.startsWith(site + '/')) throw new Error('Invalid path');
            if ((await stat(file)).isDirectory()) file = join(file, 'index.html');
            const mime: Record<string, string> = { '.html': 'text/html', '.css': 'text/css', '.svg': 'image/svg+xml', '.ttf': 'font/ttf', '.png': 'image/png', '.txt': 'text/plain' };
            res.setHeader('Content-Type', mime[extname(file)] ?? 'application/octet-stream');
            res.end(await readFile(file));
        })().catch(() => { res.statusCode = 404; res.end(); });
    });
    await new Promise<void>(yes => server.listen(0, '127.0.0.1', yes));
    const address = server.address();
    if (!address || typeof address === 'string') throw new Error('No port');
    base = `http://127.0.0.1:${address.port}`;
    directory = await mkdtemp(join(tmpdir(), 'forge-brand-'));
    await mkdir(join(directory, 'project/.git'), { recursive: true });
    child = spawn(resolve('../../target/debug/forge'), ['--json', '--state-dir', join(directory, 'state'), 'workbench', '--root', join(directory, 'project'), '--port', '0', '--app-server', resolve('../../crates/forge-cli/tests/fixtures/workbench-native.py')]);
    workbench = await new Promise<string>((yes, no) => {
        let output = '';
        const timer = setTimeout(() => no(new Error('Workbench startup timeout')), 20000);
        child.once('error', no);
        child.stdout?.on('data', (data: Buffer) => {
            output += data.toString();
            for (const line of output.split('\n')) {
                try {
                    const v: unknown = JSON.parse(line);
                    if (typeof v === 'object' && v !== null && 'url' in v && typeof v.url === 'string') {
                        clearTimeout(timer); yes(v.url);
                    }
                } catch { /* wait for complete frame */ }
            }
        });
    });
});
test.afterAll(async () => {
    child?.kill('SIGTERM');
    await new Promise<void>(yes => { if (child?.exitCode !== null) yes(); else { child?.once('exit', () => yes()); setTimeout(yes, 6000); } });
    await new Promise<void>(yes => server.close(() => yes()));
    if (directory) await rm(directory, { recursive: true, force: true });
});
for (const locale of ['pt', 'en']) for (const theme of ['light', 'dark'] as const) {
    test(`site ${locale} ${theme} desktop and mobile brand assets`, async ({ page }, info) => {
        const errors: string[] = [];
        page.on('pageerror', error => errors.push(error.message));
        await page.emulateMedia({ colorScheme: theme, reducedMotion: 'reduce' });
        for (const width of [1440, 390]) {
            await page.setViewportSize({ width, height: 960 });
            await page.goto(`${base}/${locale}/`);
            await page.evaluate(() => document.fonts.ready);
            await expect(page.locator('h1')).toBeVisible();
            expect(await page.locator('.terminal pre').evaluate(el => getComputedStyle(el).backgroundColor)).toBe('rgb(37, 37, 37)');
            expect(await page.locator('.terminal code').evaluate(el => getComputedStyle(el).color)).toBe('rgb(255, 250, 240)');
            expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
            expect(await page.evaluate(() => document.fonts.check('16px Illumi'))).toBe(true);
            expect(await page.locator('body').evaluate(el => getComputedStyle(el).backgroundColor)).toBe(theme === 'light' ? 'rgb(255, 250, 240)' : 'rgb(37, 37, 37)');
            expect(await page.locator('img').evaluateAll(images => images.every(img => (img as HTMLImageElement).complete && (img as HTMLImageElement).naturalWidth > 0))).toBe(true);
            await page.screenshot({ path: info.outputPath(`site-${locale}-${theme}-${width}.png`), fullPage: true });
        }
        expect(errors).toEqual([]);
    });
}
test('Workbench loads Illumi and the original logo without layout overflow', async ({ page }, info) => {
    await page.goto(workbench);
    await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
    await page.evaluate(() => document.fonts.ready);
    expect(await page.evaluate(() => document.fonts.check('16px Illumi'))).toBe(true);
    await expect(page.locator('.brand img')).toHaveAttribute('src', '/brand/illumitech-logo.svg');
    for (const width of [1440, 390]) {
        await page.setViewportSize({ width, height: 960 });
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
        await page.screenshot({ path: info.outputPath(`workbench-${width}.png`), fullPage: true });
    }
});
