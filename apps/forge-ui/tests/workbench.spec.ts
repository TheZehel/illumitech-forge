import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtemp, mkdir, realpath, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { messages } from '../src/conversation';
import { snapshot } from '../src/api';
let child: ChildProcess;
let directory = '';
let url = '';
let app = '';
test.beforeAll(async () => {
    directory = await realpath(await mkdtemp(join(tmpdir(), 'forge-workbench-test-')));
    app = join(directory, 'project with spaces');
    await mkdir(join(app, '.git'), { recursive: true });
    await writeFile(join(app, 'preserved.txt'), 'untouched\n');
    child = spawn(resolve('../../target/debug/illumitech-forge'), ['--json', '--state-dir', join(directory, 'state'), 'workbench', '--root', app, '--port', '0', '--app-server', resolve('../../crates/forge-cli/tests/fixtures/workbench-native.py')], { stdio: ['ignore', 'pipe', 'pipe'] });
    url = await new Promise<string>((yes, no) => { let output = ''; const timer = setTimeout(() => no(new Error('Workbench startup timeout')), 20000); child.stdout?.on('data', (b: Buffer) => { output += b.toString(); for (const line of output.split('\n'))
        try {
            const v: unknown = JSON.parse(line);
            if (typeof v === 'object' && v !== null && 'event' in v && v.event === 'workbench.started' && 'url' in v && typeof v.url === 'string') {
                clearTimeout(timer);
                yes(v.url);
            }
        }
        catch { /* partial JSONL */ } }); child.on('exit', code => { clearTimeout(timer); no(new Error(`Workbench exited ${code}`)); }); });
});
test.afterAll(async () => { if (child && child.exitCode === null) {
    child.kill('SIGTERM');
    await new Promise<void>(r => { const timer = setTimeout(() => { child.kill('SIGKILL'); r(); }, 3000); child.once('exit', () => { clearTimeout(timer); r(); }); });
} if (directory)
    await rm(directory, { recursive: true, force: true }); });
test('streamed chat, real denial, interruption, reload and isolated session controls', async ({ page }, testInfo) => {
    const errors: string[] = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.goto(url);
    await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
    await page.getByTestId('chat-input').fill('Hello fixture');
    await page.getByTestId('send-message').click();
    await expect(page.getByTestId('conversation').getByText('Native fixture reply', { exact: true })).toBeVisible();
    await expect(page.getByTestId('conversation').getByText('Resposta concluída', { exact: true })).toBeVisible();
    await page.getByRole('tab', { name: 'Consumo', exact: true }).click();
    await expect(page.getByText('18', { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Permitir uma vez' })).toHaveCount(0);
    await page.getByTestId('chat-input').fill('approval');
    await page.getByTestId('send-message').click();
    await expect(page.getByText('Permission fixture request')).toBeVisible();
    await page.getByRole('button', { name: 'Recusar', exact: true }).click();
    await expect(page.getByTestId('conversation').getByText('Não foi possível concluir a resposta', { exact: true })).toBeVisible();
    await expect(page.getByText('Decisão enviada: Recusar.', { exact: true })).toBeVisible();
    await page.getByTestId('chat-input').fill('wait');
    await page.getByTestId('send-message').click();
    await expect(page.getByTestId('interrupt-turn')).toBeVisible();
    await page.getByTestId('interrupt-turn').click();
    await expect(page.getByText('Resposta interrompida', { exact: true })).toBeVisible();
    await page.reload();
    await expect(page.getByText('Resposta interrompida', { exact: true })).toBeVisible();
    await page.getByTestId('new-session').click();
    await expect(page.getByTestId('session-list').getByRole('button')).toHaveCount(2);
    await expect(page.getByTestId('conversation').getByText('Native fixture reply', { exact: true })).toHaveCount(0);
    await page.screenshot({ path: testInfo.outputPath('workbench.png'), fullPage: true });
    expect(await readFile(join(app, 'preserved.txt'), 'utf8')).toBe('untouched\n');
    expect(errors).toEqual([]);
});
test('guided preview, apply, plan preflight and keyboard tabs', async ({ page }) => {
    await page.goto(url);
    await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
    await page.getByRole('tab', { name: 'Criar', exact: true }).click();
    await page.getByLabel('ID permanente').fill('001-browser-feature');
    await page.getByLabel('Título', { exact: true }).fill('Browser feature');
    await page.getByRole('button', { name: 'Revisar arquivos' }).click();
    await expect(page.getByText('Prévia; nenhum arquivo escrito.')).toBeVisible();
    await expect(readFile(join(app, 'specs/001-browser-feature/spec.md'))).rejects.toThrow();
    await page.getByRole('button', { name: 'Criar arquivos revisados' }).click();
    await expect(page.getByText('Arquivos criados. Aceite pendente.')).toBeVisible();
    expect(await readFile(join(app, 'specs/001-browser-feature/spec.md'), 'utf8')).toContain('Browser feature');
    await page.getByRole('tab', { name: 'Criar', exact: true }).focus();
    await page.keyboard.press('Home');
    await expect(page.getByRole('tab', { name: 'Plano', exact: true })).toBeFocused();
    await page.getByLabel('O que você quer construir?').fill('Greeting with observable output');
    await page.getByRole('button', { name: 'Prévia sem inferência' }).click();
    await expect(page.getByText('Prévia de execução', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Propor com cliente nativo' }).click();
    await expect(page.getByText('greeting-T01', { exact: true })).toBeVisible();
});
test('same-origin capability, CSRF, Host and Origin protections', async ({ page, playwright }) => {
    await page.goto(url);
    await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
    const base = new URL(url).origin;
    const isolated = await playwright.request.newContext();
    try {
        expect((await isolated.get(`${base}/api/health`)).status()).toBe(200);
        expect((await isolated.get(`${base}/api/state`)).status()).toBe(403);
        expect((await isolated.post(`${base}/api/turn`, { data: { text: 'injected' } })).status()).toBe(403);
        expect((await isolated.get(`${base}/api/health`, { headers: { Origin: 'https://evil.invalid' } })).status()).toBe(403);
        expect((await isolated.get(`${base}/api/health`, { headers: { Host: 'evil.invalid' } })).status()).toBe(403);
        const token = new URLSearchParams(new URL(url).hash.slice(1)).get('session')!;
        expect((await isolated.post(`${base}/api/connect`, { headers: { Origin: base, 'X-Forge-Token': token } })).status()).toBe(200);
        const headers = { Origin: base, 'X-Forge-CSRF': token };
        const first = await isolated.post(`${base}/api/sessions`, { headers, data: { operation_id: 'replay-guard' } });
        expect(first.status()).toBe(200);
        expect((await isolated.post(`${base}/api/sessions`, { headers, data: { operation_id: 'replay-guard' } })).status()).toBe(409);
        expect((await isolated.post(`${base}/api/resume`, { headers, data: { operation_id: 'foreign-session', thread_id: 'not-owned' } })).status()).toBe(409);
    }
    finally {
        await isolated.dispose();
    }
});
test('event reducer avoids duplicated deltas and rejects incompatible bridge data', () => {
    const value = messages([{ method: 'item/agentMessage/delta', params: { itemId: 'a', delta: 'hello' }, choices: [] }, { method: 'item/completed', params: { item: { id: 'a', type: 'agentMessage', text: 'hello' } }, choices: [] }]);
    expect(value).toHaveLength(1);
    expect(value[0].text).toBe('hello');
    expect(() => snapshot({ connected: true })).toThrow();
});

test('disconnect requires explicit reconnect and resume, without replaying a turn',async({page})=>{
 await page.goto(url);await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');await page.getByTestId('new-session').click();await page.getByTestId('chat-input').fill('disconnect');await page.getByTestId('send-message').click();await expect(page.getByTestId('connection-state')).toHaveText('Desconectado');await expect(page.getByTestId('chat-input')).toBeDisabled();await page.getByRole('button',{name:'Reconectar',exact:true}).click();await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');await expect(page.getByTestId('chat-input')).toBeDisabled();await page.getByRole('button',{name:'Retomar sessão',exact:true}).click();await expect(page.getByTestId('chat-input')).toBeEnabled();await expect(page.getByTestId('conversation').getByText('Native fixture reply',{exact:true})).toHaveCount(0);
});
test('narrow viewport keeps content reachable and tabs work by keyboard',async({page},info)=>{
 await page.setViewportSize({width:390,height:844});await page.goto(url);await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);await page.getByRole('tab',{name:'Plano',exact:true}).focus();await page.keyboard.press('ArrowRight');await expect(page.getByRole('tab',{name:'Evidências',exact:true})).toBeFocused();await page.screenshot({path:info.outputPath('workbench-mobile.png'),fullPage:true});
});
