// Offline demand + real browser/CLI regression. No live GitHub or native acceptance claim.
import { test, expect } from '@playwright/test';
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { mkdtemp, mkdir, realpath, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
let child: ChildProcess | undefined;
let directory = '';
async function launch(publication: boolean) {
    directory = await realpath(await mkdtemp(join(tmpdir(), 'forge-m08-browser-')));
    const parent = join(directory, 'repository'), root = join(directory, 'worktree'), ops = join(directory, 'operations');
    await mkdir(parent);
    for (const args of [['init', '-q', '-b', 'main'], ['config', 'user.name', 'Human Fixture'], ['config', 'user.email', 'human@example.invalid'], ['commit', '--allow-empty', '-qm', 'fixture'], ['remote', 'add', 'origin', 'https://github.com/example/project.git'], ['worktree', 'add', '-q', '-b', 'feature/m08', root]]) expect(spawnSync('git', args, { cwd: parent }).status).toBe(0);
    const source = join(directory, 'issue.json');
    await writeFile(source, JSON.stringify({ number: 42, url: 'https://github.com/example/project/issues/42', title: 'Observable greeting', body: 'Add a greeting with regression evidence.', state: 'OPEN' }));
    const cli = resolve('../../target/debug/illumitech-forge');
    const imported = spawnSync(cli, ['--json', 'issue', 'import', '--repository', 'example/project', '--number', '42', '--root', root, '--ops-root', ops, '--source-json', source, '--apply']);
    expect(imported.status, imported.stdout.toString()).toBe(0);
    const value: unknown = JSON.parse(imported.stdout.toString());
    if (typeof value !== 'object' || value === null || !('spec_id' in value) || typeof value.spec_id !== 'string') throw new Error('Invalid import response');
    const manifest = join(ops, 'releases', `${value.spec_id}.yaml`);
    if (publication) {
        // Preserve the original candidate: this is a separate reviewed test contract.
        await writeFile(join(directory, 'inspect.yaml'), (await readFile(manifest, 'utf8')).replace('publish: null', 'publish:\n    repository: example/project\n    remote: origin\n    branch: feature/m08\n    base_branch: main\n    draft: true\n    ci_authorized: false'));
    }
    child = spawn(cli, ['--json', '--state-dir', join(directory, 'state'), 'workbench', '--root', root, '--manifest', publication ? join(directory, 'inspect.yaml') : manifest, '--app-server', resolve('../../crates/forge-cli/tests/fixtures/workbench-native.py')], { stdio: ['ignore', 'pipe', 'pipe'] });
    return new Promise<string>((yes, no) => {
        let output = '', errors = '';
        const timer = setTimeout(() => no(new Error(`Startup timeout ${errors}`)), 20000);
        child?.stderr?.on('data', (bytes: Buffer) => { errors += bytes.toString(); });
        child?.stdout?.on('data', (bytes: Buffer) => { output += bytes.toString(); for (const line of output.split('\n')) try {
            const event: unknown = JSON.parse(line);
            if (typeof event === 'object' && event !== null && 'event' in event && event.event === 'workbench.started' && 'url' in event && typeof event.url === 'string') { clearTimeout(timer); yes(event.url); }
        } catch { /* partial JSONL */ } });
        child?.once('exit', code => { clearTimeout(timer); no(new Error(`Exit ${code}: ${errors}`)); });
    });
}
test.afterEach(async () => {
    const running = child;
    if (running && running.exitCode === null) { running.kill('SIGTERM'); await new Promise<void>(r => { const timer = setTimeout(() => { running.kill('SIGKILL'); r(); }, 5000); running.once('exit', () => { clearTimeout(timer); r(); }); }); }
    child = undefined;
    if (directory) await rm(directory, { recursive: true, force: true });
});
test('imported issue provenance, revision and task links are visible without acceptance', async ({ page }, testInfo) => {
    const errors: string[] = []; page.on('pageerror', error => errors.push(error.message));
    await page.goto(await launch(false));
    await page.getByRole('tab', { name: 'Evidências', exact: true }).click();
    const trace = page.getByTestId('issue-traceability');
    await expect(trace).toContainText('https://github.com/example/project/issues/42');
    await expect(trace).toContainText('offline_snapshot'); await expect(trace).toContainText('content_sha256');
    await expect(trace).toContainText('current_revisions'); await expect(trace).toContainText('-T01');
    await expect(page.getByTestId('publication-preview')).toContainText('not_authorized');
    await expect(page.getByTestId('forge-status')).toHaveText('not_run');
    await page.screenshot({ path: testInfo.outputPath('issue-provenance.png'), fullPage: true });
    expect(errors).toEqual([]);
});
test('publication contract can be inspected but cannot start execution in UI', async ({ page }, testInfo) => {
    await page.goto(await launch(true));
    await page.getByRole('tab', { name: 'Evidências', exact: true }).click();
    const preview = page.getByTestId('publication-preview');
    await expect(preview).toContainText('example/project'); await expect(preview).toContainText('Human Fixture');
    await expect(preview).toContainText('"remote_write_allowed": false'); await expect(preview).toContainText('"provider_quota": "unverified"');
    await expect(preview).toContainText('"base_branch": "main"'); await expect(preview).toContainText('"force_push": false');
    await expect(page.getByRole('button', { name: 'Iniciar execução', exact: true })).toBeDisabled();
    await page.screenshot({ path: testInfo.outputPath('publication-preview.png'), fullPage: true });
});
