// Browser regressions exercise real UI/engine and a local native-protocol fixture.
import { test, expect } from '@playwright/test';
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { mkdtemp, mkdir, realpath, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { createHash } from 'node:crypto';
let child: ChildProcess;
let directory = '', url = '', a = '', b = '', manifestPath = '';
let manifest: Record<string, unknown>;
test.setTimeout(60000);
async function launch() {
    directory = await realpath(await mkdtemp(join(tmpdir(), 'forge-m07-browser-')));
    a = join(directory, 'a'); b = join(directory, 'b');
    for (const root of [a, b]) {
        await mkdir(root);
        for (const args of [['init', '-q'], ['-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '--allow-empty', '-qm', 'fixture']]) expect(spawnSync('git', args, { cwd: root }).status).toBe(0);
        await writeFile(join(root, 'preserved.txt'), 'keep');
    }
    const gate = join(directory, 'gate.py'); await writeFile(gate, 'pass\n');
    const barrier = join(directory, 'barrier'); await mkdir(barrier);
    manifest = { schema_version: 1, release_id: 'browser-parallel', execution: { max_parallel: 2 }, repos: { a: { path: a }, b: { path: b } },
        tasks: ['a', 'b'].map(id => ({ id, repo: id, description: 'Disposable parallel browser command', max_attempts: 1, executor: { kind: 'command', argv: ['python3', '-c', `from pathlib import Path\nimport time\np=Path(${JSON.stringify(barrier)})\n(p/${JSON.stringify(id)}).write_text('ready')\nfor _ in range(2000):\n if (p/'a').exists() and (p/'b').exists(): break\n time.sleep(.01)\nelse: raise RuntimeError('missing parallel peer')\ntime.sleep(60)\n`] }, acceptance: [{ kind: 'gate', id: 'proof' }] })),
        gates: [{ id: 'proof', argv: ['python3', gate], inputs: { [gate]: createHash('sha256').update('pass\n').digest('hex') } }] };
    manifestPath = join(directory, 'release.yaml'); await writeFile(manifestPath, JSON.stringify(manifest));
    child = spawn(resolve('../../target/debug/illumitech-forge'), ['--json', '--state-dir', join(directory, 'state'), 'workbench', '--root', a, '--manifest', manifestPath, '--app-server', resolve('../../crates/forge-cli/tests/fixtures/review-native.py')], { stdio: ['ignore', 'pipe', 'pipe'] });
    url = await new Promise<string>((yes, no) => {
        let output = '', errors = ''; const timer = setTimeout(() => no(new Error(`Startup timeout ${errors}`)), 20000);
        child.stderr?.on('data', (bytes: Buffer) => { errors += bytes.toString(); });
        child.stdout?.on('data', (bytes: Buffer) => { output += bytes.toString(); for (const line of output.split('\n')) try {
            const v: unknown = JSON.parse(line); if (typeof v === 'object' && v !== null && 'event' in v && v.event === 'workbench.started' && 'url' in v && typeof v.url === 'string') { clearTimeout(timer); yes(v.url); }
        } catch { /* incomplete JSONL */ } });
        child.once('exit', code => { clearTimeout(timer); no(new Error(`Exit ${code}: ${errors}`)); });
    });
}
test.afterEach(async () => {
    if (child && child.exitCode === null) { child.kill('SIGTERM'); await new Promise<void>(r => { const timer = setTimeout(() => { child.kill('SIGKILL'); r(); }, 7000); child.once('exit', () => { clearTimeout(timer); r(); }); }); }
    if (directory) await rm(directory, { recursive: true, force: true });
});
test('UI shows parallel workers and global cancellation survives an invalidated manifest', async ({ page }, testInfo) => {
    await launch(); await page.goto(url); await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
    await page.getByRole('tab', { name: 'Evidências', exact: true }).click();
    await page.getByRole('button', { name: 'Iniciar execução', exact: true }).click();
    const tasks = page.getByRole('list', { name: 'Estados das tarefas' });
    await expect(tasks).toContainText('a: running', { timeout: 15000 }); await expect(tasks).toContainText('b: running', { timeout: 15000 });
    await expect.poll(async () => { try { return await readFile(join(directory, 'barrier/a'), 'utf8') + await readFile(join(directory, 'barrier/b'), 'utf8'); } catch { return ''; } }).toBe('readyready');
    await page.screenshot({ path: testInfo.outputPath('parallel.png'), fullPage: true });
    // A stalled read must reach its deadline and leave emergency controls usable.
    let captured!: () => void;
    const waiting = new Promise<void>(resolve => { captured = resolve; });
    let release!: () => void;
    const heldRequests = new Promise<void>(resolve => { release = resolve; });
    let holding = true;
    await page.route('**/api/forge', async route => {
        if (!holding || route.request().method() !== 'GET') { await route.continue(); return; }
        captured();
        await heldRequests;
        await route.abort().catch(() => { /* page may already be closed */ });
    });
    try {
        await waiting;
        // A later invalid contract must not remove emergency controls from the UI.
        await writeFile(manifestPath, 'invalid manifest');
        await expect(page.getByRole('alert')).toContainText('excedeu 5 segundos', { timeout: 10000 });
    } finally { holding = false; release(); }
    await page.getByRole('button', { name: 'Cancelar execução', exact: true }).click();
    await writeFile(manifestPath, JSON.stringify(manifest));
    await expect(page.getByTestId('forge-status')).toHaveText('cancelled', { timeout: 15000 });
    expect(await readFile(join(a, 'preserved.txt'), 'utf8')).toBe('keep'); expect(await readFile(join(b, 'preserved.txt'), 'utf8')).toBe('keep');
});
test('review plan, source, findings and separate usage are visible without granting acceptance', async ({ page }, testInfo) => {
    await launch(); await page.goto(url); await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
    await page.getByRole('tab', { name: 'Evidências', exact: true }).click();
    await page.getByText('Revisão nativa independente', { exact: true }).click();
    await page.getByRole('button', { name: 'Prévia da revisão', exact: true }).click();
    await expect(page.getByText('"native_call": false', { exact: false })).toBeVisible();
    await page.getByRole('button', { name: 'Executar revisão nativa', exact: true }).click();
    const review = page.locator('details').filter({ has: page.locator('summary', { hasText: 'Revisão nativa independente' }) });
    await expect(review).toContainText('"status": "completed"', { timeout: 15000 });
    await expect(review).toContainText('"acceptance": "unchanged"'); await expect(review).toContainText('"inputTokens": 40');
    await expect(page.getByTestId('forge-status')).toHaveText('not_run');
    await page.screenshot({ path: testInfo.outputPath('review.png'), fullPage: true });
});
