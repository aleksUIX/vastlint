import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { cp, mkdtemp, readFile, rename, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const project = fileURLToPath(new URL('../', import.meta.url));

async function eventually(check, logs) {
  for (let attempt = 0; attempt < 200; attempt++) {
    try {
      await check();
      return;
    } catch {
      await delay(50);
    }
  }
  assert.fail(`Extension build did not finish: ${logs()}`);
}

test('watch builds retain every runtime asset and reload atomically replaced HTML', { timeout: 30_000 }, async () => {
  const directory = await mkdtemp(join(tmpdir(), 'vastlint-chrome-build-'));
  let child;
  let logs = '';
  try {
    for (const name of ['src', 'manifest.json', 'esbuild.config.mjs']) {
      await cp(join(project, name), join(directory, name), { recursive: true });
    }
    await writeFile(join(directory, 'src/icons/.private-file'), 'excluded from extension assets');
    await symlink(join(project, 'node_modules'), join(directory, 'node_modules'), 'dir');
    child = spawn(process.execPath, ['esbuild.config.mjs', '--watch'], { cwd: directory });
    child.stdout.on('data', (data) => { logs = (logs + data).slice(-8000); });
    child.stderr.on('data', (data) => { logs = (logs + data).slice(-8000); });
    const source = await readFile(join(directory, 'manifest.json'), 'utf8');
    const manifest = JSON.parse(source);
    const runtimeFiles = new Set([
      manifest.action.default_popup,
      manifest.background.service_worker,
      ...Object.values(manifest.icons),
      ...manifest.content_scripts.flatMap((entry) => entry.js),
      'analysis.html', 'analysis.js', 'popup.js', 'vastlint_wasm_bg.wasm',
    ]);
    await eventually(async () => {
      assert.equal(await readFile(join(directory, 'dist/manifest.json'), 'utf8'), source);
      for (const name of runtimeFiles) assert.ok((await readFile(join(directory, 'dist', name))).length);
    }, () => logs);
    assert.deepEqual(
      await readFile(join(directory, 'dist/vastlint_wasm_bg.wasm')),
      await readFile(join(project, 'node_modules/vastlint/vastlint_wasm_bg.wasm')),
    );
    await assert.rejects(readFile(join(directory, 'dist/icons/.private-file')), { code: 'ENOENT' });
    for (const label of ['first', 'second']) {
      const html = `<p>${label} static asset update</p>`;
      const target = join(directory, 'src/popup/popup.html');
      await writeFile(`${target}.tmp`, html);
      await rename(`${target}.tmp`, target);
      await eventually(async () => {
        assert.equal(await readFile(join(directory, 'dist/popup.html'), 'utf8'), html);
      }, () => logs);
    }
  } finally {
    if (child && child.exitCode === null && child.signalCode === null) {
      const stopped = once(child, 'exit');
      child.kill('SIGTERM');
      await stopped;
    }
    await rm(directory, { recursive: true, force: true });
  }
});
