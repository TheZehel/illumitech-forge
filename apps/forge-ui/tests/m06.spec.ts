// Local browser/engine regressions; native protocol fixture is not provider acceptance.
import { test, expect } from '@playwright/test';
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { mkdtemp, mkdir, realpath, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { createHash } from 'node:crypto';
let child: ChildProcess;
let directory = '', url = '', app = '';
test.setTimeout(60000);
async function launch(budget: Record<string, unknown> = {}, human = true) {
    directory = await realpath(await mkdtemp(join(tmpdir(), 'forge-m06-browser-')));
    app = join(directory, 'project'); await mkdir(app);
    for (const args of [['init', '-q'], ['-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '--allow-empty', '-qm', 'fixture']]) expect(spawnSync('git', args, { cwd: app }).status).toBe(0);
    await writeFile(join(app, 'preserved.txt'), 'keep');
    const gate = join(directory, 'gate.py'); await writeFile(gate, 'pass\n');
    const manifest = { schema_version: 1, release_id: 'browser-controls', repos: { app: { path: app } }, tasks: [{ id: 'work', repo: 'app', description: 'Browser fixture command', executor: { kind: 'command', argv: ['python3', '-c', "from pathlib import Path; Path('done.txt').write_text('done')"] }, acceptance: [{ kind: 'gate', id: 'proof' }, ...(human ? [{ kind: 'human', id: 'review' }] : [])] }], gates: [{ id: 'proof', argv: ['python3', gate], inputs: { [gate]: createHash('sha256').update('pass\n').digest('hex') }, cache: { enabled: true, ttl_seconds: 300 } }], budget };
    const path = join(directory, 'release.yaml'); await writeFile(path, JSON.stringify(manifest));
    child = spawn(resolve('../../target/debug/illumitech-forge'), ['--json', '--state-dir', join(directory, 'state'), 'workbench', '--root', app, '--port', '0', '--manifest', path, '--app-server', resolve('../../crates/forge-cli/tests/fixtures/workbench-native.py')], { stdio: ['ignore', 'pipe', 'pipe'] });
    url = await new Promise<string>((yes, no) => { let output = '', errors = ''; const timer = setTimeout(() => no(new Error(`Startup timeout ${errors}`)), 20000); child.stderr?.on('data', b => errors += String(b)); child.stdout?.on('data', (b: Buffer) => { output += b.toString(); for (const line of output.split('\n')) try { const v: unknown = JSON.parse(line); if (typeof v === 'object' && v !== null && 'event' in v && v.event === 'workbench.started' && 'url' in v && typeof v.url === 'string') { clearTimeout(timer); yes(v.url); } } catch { /* incomplete frame */ } }); child.on('exit', code => { clearTimeout(timer); no(new Error(`Exit ${code}: ${errors}`)); }); });
}
test.afterEach(async () => {
    if (child && child.exitCode === null) { child.kill('SIGTERM'); await new Promise<void>(r => { const timer = setTimeout(() => { child.kill('SIGKILL'); r(); }, 7000); child.once('exit', () => { clearTimeout(timer); r(); }); }); }
    if (directory) await rm(directory, { recursive: true, force: true });
});
test('authentic UI starts gates and binds denial and approval to human review', async ({ page }) => {
    await launch(); await page.goto(url); await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
    await page.getByRole('tab', { name: 'Evidências', exact: true }).click();
    await page.getByRole('button', { name: 'Iniciar execução', exact: true }).click();
    await expect(page.getByTestId('forge-status')).toHaveText('awaiting_approval', { timeout: 15000 });
    expect(await readFile(join(app, 'done.txt'), 'utf8')).toBe('done');
    await page.getByLabel('Identificação de quem revisou').fill('Fixture reviewer');
    await page.getByRole('button', { name: 'Recusar revisão', exact: true }).click();
    await expect(page.getByText('Revisão recusada. Nenhuma aprovação concedida.')).toBeVisible();
    // Delay a read begun before resume so it arrives after the write acknowledgement.
    // This read must never re-enable approval while the resumed engine still owns the lease.
    let captured!: () => void, release!: () => void;
    const ready = new Promise<void>(r => { captured = r; });
    const delayed = new Promise<void>(r => { release = r; });
    let hold = true;
    await page.route('**/api/forge', async route => {
        if (!hold) { await route.continue(); return; }
        hold = false;
        const response = await route.fetch(); captured(); await delayed; await route.fulfill({ response });
    });
    await ready;
    const resumed = page.waitForResponse(r => r.url().endsWith('/api/forge-resume'));
    await page.getByRole('button', { name: 'Retomar execução', exact: true }).click();
    await resumed; release();
    await expect(page.getByTestId('forge-status')).toHaveText('awaiting_approval', { timeout: 15000 });
    await page.getByLabel('Revisei o diff e o conteúdo desta revisão').check();
    const response = page.waitForResponse(r => r.url().endsWith('/api/forge-approve'));
    await page.getByRole('button', { name: 'Aprovar revisão', exact: true }).click();
    expect(await (await response).json()).toMatchObject({ approved: true });
    await expect(page.getByText('Aprovação registrada para esta revisão. Retome explicitamente.')).toBeVisible();
    await page.getByRole('button', { name: 'Retomar execução', exact: true }).click();
    await expect(page.getByTestId('forge-status')).toHaveText('passed', { timeout: 15000 });
    expect(await readFile(join(app, 'preserved.txt'), 'utf8')).toBe('keep');
});
test('forged origin, duplicates, unknown fields and stale tokens cannot start a second run', async ({ page, request }) => {
    await launch({}, false); await page.goto(url); await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
    const origin = new URL(url).origin, token = new URLSearchParams(new URL(url).hash.slice(1)).get('session') ?? '';
    const headers = { Origin: origin, Cookie: `forge_session=${token}`, 'X-Forge-CSRF': token };
    const inspection = await request.get(`${origin}/api/forge`, { headers }); expect(inspection.ok()).toBeTruthy(); const v: { control_revision: string } = await inspection.json();
    expect((await request.post(`${origin}/api/forge-start`, { headers: { ...headers, Origin: 'https://evil.invalid' }, data: { operation_id: 'forged', revision: v.control_revision } })).ok()).toBeFalsy();
    expect((await request.post(`${origin}/api/forge-start`, { headers: { Origin: origin }, data: { operation_id: 'no-session', revision: v.control_revision } })).ok()).toBeFalsy();
    await writeFile(join(app, 'changed.txt'), 'source changed');
    expect((await request.post(`${origin}/api/forge-start`, { headers, data: { operation_id: 'old', revision: v.control_revision } })).ok()).toBeFalsy();
    const current: { control_revision: string } = await (await request.get(`${origin}/api/forge`, { headers })).json(); const body = { operation_id: 'one-start', revision: current.control_revision };
    expect((await request.post(`${origin}/api/forge-start`, { headers, data: body })).ok()).toBeTruthy();
    expect((await request.post(`${origin}/api/forge-start`, { headers, data: body })).ok()).toBeFalsy();
    expect((await request.post(`${origin}/api/forge-start`, { headers, data: { operation_id: 'unknown', revision: current.control_revision, model: 'other' } })).ok()).toBeFalsy();
});
test('budget exhaustion is visible before command spawn', async ({ page }) => {
    await launch({ max_attempts: 0 }); await page.goto(url); await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
    await page.getByRole('tab', { name: 'Evidências', exact: true }).click(); await page.getByRole('button', { name: 'Iniciar execução', exact: true }).click();
    await expect(page.getByTestId('forge-status')).toHaveText('blocked'); await expect(readFile(join(app, 'done.txt'))).rejects.toThrow();
});
