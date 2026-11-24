import { afterEach, test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { projectPlan } from './cli.mjs';
import { options } from './process.mjs';

const directories = [];
const temp = () => { const path = mkdtempSync(join(tmpdir(), 'furigana-ci-test-')); directories.push(path); return path; };
afterEach(() => { for (const path of directories.splice(0)) rmSync(path, { recursive: true, force: true }); });

test('missing targets fail instead of producing a synthetic green result', () => {
  const path = join(temp(), 'config.json');
  writeFileSync(path, JSON.stringify({ schemaVersion: 1, targets: [] }));
  assert.throws(() => projectPlan(path));
});
test('repository plan declares the real project targets', () => {
  assert.deepEqual(projectPlan().targets, ['lint', 'test', 'build']);
});
test('argv options reject duplicates and preserve literal shell metacharacters', () => {
  assert.deepEqual(options(['--name', '$(id); example']), { name: '$(id); example' });
  assert.throws(() => options(['--name', 'a', '--name', 'b']));
});
