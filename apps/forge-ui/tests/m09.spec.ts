import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdtemp, mkdir, realpath, copyFile, chmod, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';

const execute = promisify(execFile);
let child: ChildProcess | undefined;
let directory = '';
async function stop(): Promise<void> {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  const process = child;
  await new Promise<void>(done => {
    const timer = setTimeout(() => { process.kill('SIGKILL'); }, 3000);
    process.once('exit', () => { clearTimeout(timer); done(); });
    process.kill('SIGTERM');
  });
}
test.afterEach(async () => {
  await stop();
  if (directory) await rm(directory, { recursive: true, force: true });
});
test('standalone installed binary serves embedded UI and excludes live workbench snapshots', async ({ page }) => {
  directory = await realpath(await mkdtemp(join(tmpdir(), 'forge-standalone-')));
  const installation = join(directory, 'installation');
  const binary = join(installation, 'illumitech-forge');
  const root = join(directory, 'project');
  const state = join(directory, 'state');
  const native = join(directory, 'protocol-fixture.py');
  await mkdir(installation);
  await mkdir(join(root, '.git'), { recursive: true });
  await copyFile(resolve('../../target/debug/illumitech-forge'), binary);
  await copyFile(resolve('../../crates/forge-cli/tests/fixtures/workbench-native.py'), native);
  await chmod(binary, 0o700); await chmod(native, 0o700);
  // No repository ancestor, Node runtime dependency, UI dist directory or source fallback.
  child = spawn(binary, ['--json', '--state-dir', state, 'workbench', '--root', root, '--app-server', native], { cwd: directory, stdio: ['ignore', 'pipe', 'pipe'] });
  const process = child;
  const url = await new Promise<string>((yes, no) => {
    let output = '';
    const timer = setTimeout(() => no(new Error('Standalone workbench startup timeout')), 15000);
    process.on('error', no);
    process.on('exit', code => { clearTimeout(timer); no(new Error(`Standalone workbench exited ${code}`)); });
    process.stdout?.on('data', (bytes: Buffer) => {
      output += bytes.toString();
      for (const line of output.split('\n')) {
        try {
          const value: unknown = JSON.parse(line);
          if (typeof value === 'object' && value !== null && 'event' in value && value.event === 'workbench.started' && 'url' in value && typeof value.url === 'string') {
            clearTimeout(timer); yes(value.url);
          }
        } catch { /* partial JSONL */ }
      }
    });
  });
  const errors: string[] = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto(url);
  await expect(page.getByTestId('connection-state')).toHaveText('Conectado ao nativo');
  await expect(page.getByTestId('chat-input')).toBeVisible();
  const snapshot = join(directory, 'snapshot');
  await expect(execute(binary, ['--json', '--state-dir', state, 'state', 'snapshot', '--output', snapshot], { cwd: directory })).rejects.toThrow();
  await stop();
  const result = await execute(binary, ['--json', '--state-dir', state, 'state', 'snapshot', '--output', snapshot], { cwd: directory });
  const value: unknown = JSON.parse(result.stdout);
  expect(value).toMatchObject({ status: 'snapshotted', restore_probe: 'passed' });
  const restored = await execute(binary, ['--json', 'state', 'restore', '--snapshot', snapshot, '--target-state-dir', join(directory, 'restored'), '--apply'], { cwd: directory });
  expect(JSON.parse(restored.stdout) as unknown).toMatchObject({ status: 'restored' });
  expect(errors).toEqual([]);
});
