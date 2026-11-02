import { test, expect } from '@playwright/test';
import { spawn, execFileSync, type ChildProcess } from 'node:child_process';
import { mkdtemp, mkdir, realpath, writeFile, readFile, chmod, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
let child: ChildProcess; let directory = ''; let app = ''; let url = '';
test.beforeAll(async () => {
    directory = await realpath(await mkdtemp(join(tmpdir(), 'forge-growth-browser-'))); app = join(directory, 'project');
    await mkdir(app); execFileSync('git', ['init', '-q'], { cwd: app });
    execFileSync('git', ['-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgSign=false', 'commit', '--allow-empty', '-qm', 'fixture'], { cwd: app });
    await writeFile(join(app, 'preserved.txt'), 'keep');
    const bin = join(directory, 'bin'); await mkdir(bin);
    await writeFile(join(bin, 'codex'), '#!/usr/bin/env python3\nimport json\nfrom pathlib import Path\nPath("output.txt").write_text("verified")\nprint(json.dumps({"type":"thread.started","thread_id":"fixture"}))\nprint(json.dumps({"type":"turn.completed","usage":{"input_tokens":10,"cached_input_tokens":0,"output_tokens":2}}))\n'); await chmod(join(bin, 'codex'), 0o700);
    child = spawn(resolve('../../target/debug/forge'), ['--json', '--state-dir', join(directory, 'state'), 'workbench', '--root', app, '--port', '0', '--app-server', resolve('../../crates/forge-cli/tests/fixtures/workbench-native.py')], { stdio: ['ignore', 'pipe', 'pipe'], env: { ...process.env, PATH: `${bin}:${process.env.PATH ?? ''}` } });
    url = await new Promise<string>((yes, no) => { let output = ''; const timer = setTimeout(() => no(new Error('startup timeout')), 20000); child.stdout?.on('data', d => { output += String(d); for (const line of output.split('\n')) { try { const v: unknown = JSON.parse(line); if (typeof v === 'object' && v !== null && 'url' in v && typeof v.url === 'string') { clearTimeout(timer); yes(v.url); } } catch { /* wait for a complete frame */ } } }); child.once('exit', code => { clearTimeout(timer); no(new Error(`server exited ${code}`)); }); });
});
test.afterAll(async () => { child?.kill('SIGTERM'); await new Promise<void>(yes => { if (child?.exitCode !== null) yes(); else { child?.once('exit', () => yes()); setTimeout(yes, 6000); } }); if (directory) await rm(directory, { recursive: true, force: true }); });
test('one scope consent executes, reports and survives reload without replay', async ({ page }) => {
    await page.goto(url); await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
    await page.getByRole('button', { name: 'Executar uma tarefa', exact: true }).click();
    await page.getByLabel('Resultado esperado e limites').fill('Create output.txt with verified and preserve preserved.txt.');
    await page.getByText('Verificações e regressão externa', { exact: true }).click();
    await page.getByLabel('Checks (JSON)').fill(JSON.stringify([{ id: 'output', argv: ['python3', '{checks}/verify.py', '{repo}'], timeout_seconds: 10 }]));
    await page.getByLabel('Arquivos de regressão').fill(JSON.stringify({ 'verify.py': 'import pathlib,sys\np=pathlib.Path(sys.argv[1]);assert (p/"output.txt").read_text()=="verified";assert (p/"preserved.txt").read_text()=="keep"\n' }));
    await page.getByRole('button', { name: 'Revisar escopo', exact: true }).click(); await expect(page.getByRole('heading', { name: 'Escopo revisável' })).toBeVisible();
    await expect(readFile(join(app, 'specs/001-minha-tarefa/spec.md'))).rejects.toThrow();
    await page.getByRole('button', { name: 'Salvar candidato externo' }).click();
    const run = page.getByRole('button', { name: 'Aprovar escopo e executar' }); await expect(run).toBeDisabled();
    await page.getByLabel('Revisei o plano e os verificadores.').check(); await expect(run).toBeEnabled(); await run.click();
    try { await expect(page.getByText('Estado: passed', { exact: true })).toBeVisible({ timeout: 20000 }); } catch (error: unknown) { await test.info().attach('workflow-diagnostic', { body: await page.locator('.workflow-view').innerText(), contentType: 'text/plain' }); await page.getByText('Relatório e evidências', { exact: true }).click(); await test.info().attach('workflow-report', { body: await page.locator('.workflow-view').innerText(), contentType: 'text/plain' }); throw error; }
    expect(await readFile(join(app, 'preserved.txt'), 'utf8')).toBe('keep');
    await page.reload(); await page.getByRole('button', { name: 'Executar uma tarefa', exact: true }).click();
    await page.getByLabel('Retomar tarefa existente').selectOption('001-minha-tarefa');
    await expect(page.getByText('Estado: passed', { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Aprovar escopo e executar' })).toBeDisabled();
    await writeFile(join(app, 'output.txt'), 'stale');
    await page.getByRole('button', { name: 'Atualizar diagnóstico' }).click(); await expect(page.getByText('Estado: blocked', { exact: true })).toBeVisible();
    await page.getByRole('tab', { name: 'Executar', exact: true }).focus(); await page.keyboard.press('Home'); await expect(page.getByRole('tab', { name: 'Plano', exact: true })).toBeFocused();
});
