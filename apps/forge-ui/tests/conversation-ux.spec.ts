import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtemp, mkdir, realpath, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { messages } from '../src/conversation';
import { diffCounts } from '../src/activity';

let child: ChildProcess;
let directory = '';
let url = '';
test.beforeAll(async () => {
    directory = await realpath(await mkdtemp(join(tmpdir(), 'forge-chat-ux-')));
    await mkdir(join(directory, 'project/.git'), { recursive: true });
    child = spawn(resolve('../../target/debug/illumitech-forge'), ['--json', '--state-dir', join(directory, 'state'), 'workbench', '--root', join(directory, 'project'), '--port', '0', '--app-server', resolve('../../crates/forge-cli/tests/fixtures/workbench-native.py')]);
    url = await new Promise<string>((yes, no) => {
        let output = '';
        const timer = setTimeout(() => no(new Error('Workbench startup timeout')), 20000);
        child.on('error', no);
        child.on('exit', code => { clearTimeout(timer); no(new Error(`Workbench exited ${code}`)); });
        child.stdout?.on('data', (data: Buffer) => {
            output += data.toString();
            for (const line of output.split('\n')) try {
                const v = JSON.parse(line) as { event?: string; url?: string };
                if (v.event === 'workbench.started' && v.url) { clearTimeout(timer); yes(v.url); }
            } catch { /* partial JSONL */ }
        });
    });
});
test.afterAll(async () => {
    if (child?.exitCode === null) { child.kill('SIGTERM'); await new Promise<void>(r => child.once('exit', () => r())); }
    if (directory) await rm(directory, { recursive: true, force: true });
});
const event = (method: string, params: Record<string, unknown>) => ({ method, params });
const user = (text: string) => event('bridge/user', { text });
const answer = (text: string, id = 'a') => event('item/completed', { item: { id, type: 'agentMessage', text } });
function state(events: unknown[] = [], active: string | null = null, loaded = true) {
    return { schema_version: 1, connected: true, error: '', project: { name: 'Meu projeto' }, account: { authenticated: true },
        sessions: [{ id: 'test-thread', title: 'Uma conversa', loaded, active_turn: active, events, usage: {}, effective: {} }], approvals: [], forge: {} };
}

test('initial skeleton waits for state and reduced motion disables animation', async ({ page }) => {
    let release!: () => void;
    const waiting = new Promise<void>(r => { release = r; });
    await page.route('**/api/state', async route => { await waiting; await route.fulfill({ json: state() }); });
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto(url);
    await expect(page.getByTestId('conversation-skeleton')).toBeVisible();
    await expect(page.getByText('O próximo passo começa')).toHaveCount(0);
    expect(await page.locator('.skeleton').first().evaluate(e => getComputedStyle(e).animationName)).toBe('none');
    release();
    await expect(page.getByTestId('conversation-skeleton')).toHaveCount(0);
    await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
});

test('safe Markdown renders rich text without active HTML, executable links or remote images', async ({ page }, info) => {
    const markdown = '## Vamos construir\n\nUma resposta **clara**.\n\n- Primeiro passo\n- Segundo passo\n\n```rust\nfn main() { println!("Olá"); }\n```\n\n| Tarefa | Estado |\n| --- | --- |\n| Revisão | Pronta |\n\n[Perigoso](javascript:alert(1))\n\n[Documentação](https://example.invalid/docs)\n\n![Imagem](https://tracking.invalid/pixel)\n\n<script>window.injected = true</script>';
    const remote: string[] = [];
    page.on('request', r => { if (r.url().includes('tracking.invalid')) remote.push(r.url()); });
    await page.route('**/api/state', r => r.fulfill({ json: state([answer(markdown)]) }));
    await page.goto(url);
    await expect(page.getByRole('heading', { name: 'Vamos construir' })).toBeVisible();
    await expect(page.locator('.markdown strong')).toHaveText('clara');
    await expect(page.locator('.markdown li')).toHaveCount(2);
    await expect(page.locator('.markdown table')).toContainText('Pronta');
    await expect(page.locator('.markdown pre code')).toContainText('fn main()');
    await expect(page.locator('a[href^="javascript:"]')).toHaveCount(0);
    await expect(page.locator('.markdown img, .markdown script')).toHaveCount(0);
    expect(remote).toEqual([]);
    await expect(page.getByRole('link', { name: 'Documentação' })).toHaveAttribute('rel', 'noopener noreferrer');
    await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
    await page.getByRole('button', { name: 'Copiar código' }).click();
    await expect(page.getByRole('button', { name: 'Copiado' })).toBeVisible();
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('fn main() { println!("Olá"); }');
    await page.screenshot({ path: info.outputPath('conversation-markdown.png'), fullPage: true });
    await page.setViewportSize({ width: 390, height: 844 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath('conversation-mobile.png'), fullPage: true });
});

test('send is immediate, appears once after native echo and blocks keyboard replay while active', async ({ page }) => {
    let current = state();
    let sends = 0;
    let release!: () => void;
    const waiting = new Promise<void>(r => { release = r; });
    await page.route('**/api/state', r => r.fulfill({ json: current }));
    await page.route('**/api/turn', async r => {
        sends++; await waiting;
        current = state([user('Meu pedido'), event('item/completed', { item: { id: 'native-user', type: 'userMessage', content: [{ type: 'text', text: 'Meu pedido' }] } })], 'turn-1');
        await r.fulfill({ json: { ok: true } });
    });
    await page.goto(url);
    await page.getByTestId('chat-input').fill('Meu pedido');
    await page.getByTestId('send-message').click();
    await expect(page.getByTestId('response-progress')).toContainText('Enviando mensagem');
    await expect(page.getByTestId('conversation').getByText('Meu pedido', { exact: true })).toHaveCount(1);
    release();
    await expect(page.getByTestId('interrupt-turn')).toBeVisible();
    await expect(page.getByTestId('conversation').getByText('Meu pedido', { exact: true })).toHaveCount(1);
    await page.getByTestId('chat-input').fill('Próximo pedido');
    await page.getByTestId('chat-input').press('Control+Enter');
    expect(sends).toBe(1);
    current = state([...current.sessions[0].events, answer('Vamos por partes.'), event('turn/completed', { turn: { status: 'completed' } })]);
    await expect(page.getByText('Resposta concluída', { exact: true })).toBeVisible();
    await expect(page.getByTestId('response-progress')).toHaveCount(0);
    await expect(page.getByTestId('chat-input')).toHaveValue('Próximo pedido');
});

test('uncertain write failure ends loading without retrying or losing the draft', async ({ page }) => {
    let writes = 0;
    await page.route('**/api/state', r => r.fulfill({ json: state() }));
    await page.route('**/api/turn', r => { writes++; return r.fulfill({ status: 503, json: { error: 'Resultado desconhecido' } }); });
    await page.goto(url);
    await page.getByTestId('chat-input').fill('Preserve meu pedido');
    await page.getByTestId('send-message').click();
    await expect(page.getByRole('alert')).toContainText('Resultado desconhecido');
    await expect(page.getByTestId('response-progress')).toHaveCount(0);
    await expect(page.getByTestId('chat-input')).toHaveValue('Preserve meu pedido');
    // A fresh successful state poll cannot dismiss an uncertain mutation outcome.
    await page.waitForResponse(r => r.url().endsWith('/api/state'));
    await expect(page.getByRole('alert')).toContainText('Resultado desconhecido');
    expect(writes).toBe(1);
});

test('loaded selection avoids resume; unloaded selection shows skeleton until explicit resume resolves', async ({ page }) => {
    let current = state([answer('Histórico preservado')]);
    let resumes = 0;
    let release!: () => void;
    const waiting = new Promise<void>(r => { release = r; });
    await page.route('**/api/state', r => r.fulfill({ json: current }));
    await page.route('**/api/resume', async r => { resumes++; await waiting; current = state([answer('Histórico retomado')]); await r.fulfill({ json: { ok: true } }); });
    await page.goto(url);
    await page.getByTestId('session-list').getByRole('button').click();
    expect(resumes).toBe(0);
    current = state([], null, false);
    await expect(page.getByRole('button', { name: 'Retomar sessão' })).toBeVisible();
    await page.getByRole('button', { name: 'Retomar sessão' }).click();
    await expect(page.getByTestId('conversation-skeleton')).toBeVisible();
    await expect(page.getByTestId('chat-input')).toBeDisabled();
    release();
    await expect(page.getByText('Histórico retomado', { exact: true })).toBeVisible();
    await expect(page.getByTestId('conversation-skeleton')).toHaveCount(0);
    expect(resumes).toBe(1);
});

test('new streaming output does not pull the reader away from history', async ({ page }) => {
    const history = Array.from({ length: 24 }, (_, i) => answer(`Resposta ${i}\n\n${'Conteúdo para leitura. '.repeat(30)}`, `a-${i}`));
    let current = state(history, 'active');
    await page.route('**/api/state', r => r.fulfill({ json: current }));
    await page.goto(url);
    const conversation = page.getByTestId('conversation');
    await expect(page.getByText('Resposta 23', { exact: false })).toBeVisible();
    await conversation.evaluate(e => { e.scrollTop = 160; e.dispatchEvent(new Event('scroll')); });
    current = state([...history, event('item/agentMessage/delta', { itemId: 'new', delta: 'Nova resposta chegando' })], 'active');
    await expect(page.getByRole('button', { name: 'Ir para a resposta mais recente' })).toBeVisible();
    expect(await conversation.evaluate(e => e.scrollTop)).toBeCloseTo(160, 0);
    await page.getByRole('button', { name: 'Ir para a resposta mais recente' }).click();
    expect(await conversation.evaluate(e => e.scrollHeight - e.clientHeight - e.scrollTop)).toBeLessThan(5);
});

test('native user echo collapses within a turn while repeated prompts across turns stay distinct', () => {
    const input = (id: string) => ({ method: 'item/completed', params: { item: { id, type: 'userMessage', text: 'Mesmo pedido' } }, choices: [] });
    const events = [user('Mesmo pedido'), input('u1'), event('turn/completed', { turn: { status: 'completed' } }), user('Mesmo pedido'), input('u2')].map(e => ({ ...e, choices: [] }));
    expect(messages(events).filter(m => m.kind === 'userMessage')).toHaveLength(2);
    expect(messages([input('history-u1'), input('history-u2')]).filter(m => m.kind === 'userMessage')).toHaveLength(2);
});

test('activity lifecycle merges partial output by ID and preserves structured tool errors', () => {
    const input = [event('item/started', { item: { id: 'command', type: 'commandExecution', command: 'cargo check', cwd: '/project' } }),
        event('item/commandExecution/outputDelta', { itemId: 'command', delta: 'Compiling\n' }),
        event('item/commandExecution/outputDelta', { itemId: 'command', delta: 'Done\n' })].map(e => ({ ...e, choices: [] }));
    const partial = messages(input);
    expect(partial).toHaveLength(1);
    expect(partial[0].status).toBe('inProgress');
    expect(partial[0].activity?.sections.find(s => s.label === 'Saída')?.text).toBe('Compiling\nDone\n');
    const final = messages([...input, { ...event('item/completed', { item: { id: 'command', type: 'commandExecution', status: 'completed', exitCode: 0, aggregatedOutput: 'Final\n' } }), choices: [] }]);
    expect(final).toHaveLength(1);
    expect(final[0].activity?.sections.find(s => s.label === 'Comando')?.text).toBe('cargo check');
    expect(final[0].activity?.sections.find(s => s.label === 'Saída')?.text).toBe('Final\n');
    const failed = messages([{ ...event('item/completed', { item: { id: 'mcp', type: 'mcpToolCall', server: 'logs', tool: 'query', status: 'completed', arguments: { limit: 3 }, result: { isError: true, content: [{ text: 'Unavailable' }] } } }), choices: [] }]);
    expect(failed[0].status).toBe('failed');
    expect(failed[0].activity?.title).toBe('Chamada · logs.query');
    expect(failed[0].activity?.sections.map(s => s.label)).toEqual(['Argumentos', 'Resultado']);
    expect(diffCounts('--- a\n+++ b\n@@ -1 +1,2 @@\n-old\n+new\n+++ literal\n')).toEqual({ added: 2, removed: 1 });
    expect(diffCounts('+++ header\n+text without a hunk')).toEqual({});
    const truncated = messages([{ ...event('item/completed', { item: { id: 'files', type: 'fileChange', forgeOriginalFileCount: 100, forgeDisplayTruncated: true, changes: [{ path: 'a.rs', diff: '@@ -0,0 +1 @@\n+new\n' }, '… omitted'] } }), choices: [] }]);
    expect(truncated[0].activity?.files).toHaveLength(1);
    expect(truncated[0].activity?.title).toContain('1 de 100 arquivos');
});

test('MCP details are keyboard accessible, safe, and remain open across streaming updates', async ({ page }, info) => {
    const start = event('item/started', { item: { id: 'mcp', type: 'mcpToolCall', server: 'observability', tool: 'list_logs', arguments: { status: 500, limit: 5 } } });
    let current = state([start], 'active');
    await page.route('**/api/state', r => r.fulfill({ json: current }));
    await page.goto(url);
    const card = page.getByTestId('activity-card');
    await expect(card).toHaveCount(1);
    const summary = card.locator('summary').first();
    await expect(summary).toContainText('Chamada · observability.list_logs');
    await expect(summary).toContainText('Em andamento');
    await expect(card.getByRole('heading', { name: 'Argumentos' })).toHaveCount(0);
    await summary.focus(); await page.keyboard.press('Enter');
    await expect(summary).toContainText('Ocultar detalhes');
    await expect(card.getByRole('heading', { name: 'Argumentos' })).toBeVisible();
    current = state([start, event('item/mcpToolCall/progress', { itemId: 'mcp', message: 'Consultando eventos' })], 'active');
    await expect(card.getByText('Consultando eventos', { exact: true })).toBeVisible();
    current = state([start, event('item/completed', { item: { id: 'mcp', type: 'mcpToolCall', status: 'failed', result: { events: [{ message: '<img src="https://invalid.example/pixel" onerror="alert(1)">' }] }, error: { message: 'Consulta incompleta' } } })]);
    await expect(card).toHaveCount(1);
    await expect(summary).toContainText('Falhou');
    await expect(card.getByRole('heading', { name: 'Resultado' })).toBeVisible();
    await expect(card.getByRole('heading', { name: 'Erro', exact: true })).toBeVisible();
    await expect(card.locator('img, script')).toHaveCount(0);
    await expect(card.getByRole('heading', { name: 'Argumentos' })).toBeVisible();
    await page.screenshot({ path: info.outputPath('mcp-expanded.png'), fullPage: true });
    await summary.press('Space');
    await expect(summary).toContainText('Mostrar detalhes');
    await expect(card.getByRole('heading', { name: 'Resultado' })).toHaveCount(0);
});

test('file edits show received line counts and individual collapsible diffs', async ({ page }, info) => {
    await page.route('**/api/state', r => r.fulfill({ json: state([event('item/completed', { item: { id: 'files', type: 'fileChange', status: 'completed', changes: [
        { path: 'src/example.rs', kind: { type: 'update' }, diff: '--- a/src/example.rs\n+++ b/src/example.rs\n@@ -1 +1,2 @@\n-old\n+new\n+second\n' },
        { path: 'README.md', kind: { type: 'add' }, diff: '@@ -0,0 +1 @@\n+# Example\n' },
    ] } })]) }));
    await page.goto(url);
    const card = page.getByTestId('activity-card');
    await expect(card.locator('summary').first()).toContainText('2 arquivos (+3 −1)');
    await card.locator('summary').first().click();
    const file = card.locator('.activity-file').first();
    await expect(file.locator('summary')).toContainText('src/example.rs');
    await expect(file.locator('pre')).toBeHidden();
    await file.locator('summary').click();
    await expect(file.locator('pre')).toBeVisible();
    await expect(file.locator('pre')).toContainText('+second');
    await expect(card).toContainText('Alterações não comprovam aceite');
    await page.setViewportSize({ width: 390, height: 844 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath('file-details-mobile.png'), fullPage: true });
});

test('large activity output is bounded and command metadata stays separate', async ({ page }) => {
    await page.route('**/api/state', r => r.fulfill({ json: state([event('item/completed', { item: { id: 'cmd', type: 'commandExecution', command: 'cargo test', cwd: '/project', status: 'completed', exitCode: 0, aggregatedOutput: 'line\n'.repeat(8000), forgeDisplayTruncated: true } })]) }));
    await page.goto(url);
    const card = page.getByTestId('activity-card');
    await expect(card.locator('summary').first()).toContainText('Comando · cargo test · parcial');
    await expect(card.locator('pre')).toHaveCount(0);
    await card.locator('summary').first().click();
    await expect(card.getByRole('heading', { name: 'Código de saída' })).toBeVisible();
    await page.waitForResponse(r => r.url().endsWith('/api/state'));
    await expect(card.locator('summary').first()).toContainText('Ocultar detalhes');
    await expect(card).toContainText('Exibição limitada a 24000 caracteres');
    const output = card.getByRole('region', { name: 'Saída', exact: true }).locator('pre');
    expect((await output.textContent())!.length).toBeLessThan(24200);
    expect(await output.evaluate(e => e.scrollHeight > e.clientHeight)).toBe(true);
});
