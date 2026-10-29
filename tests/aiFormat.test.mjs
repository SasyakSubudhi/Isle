import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/modules/ai/aiFormat.ts', import.meta.url), 'utf8');
const js = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { formatTokens, until, ago, windowName } = await import(
  `data:text/javascript;base64,${Buffer.from(js).toString('base64')}`
);

test('token counts are short', () => {
  assert.equal(formatTokens(850), '850');
  assert.equal(formatTokens(1234), '1.2K');
  assert.equal(formatTokens(48_000), '48K');
  assert.equal(formatTokens(3_400_000), '3.4M');
  assert.equal(formatTokens(24_000_000), '24M');
});

test('reset countdowns', () => {
  const now = 1_000_000_000;
  assert.equal(until(now + 5 * 60_000, now), 'in 5m');
  assert.equal(until(now + (2 * 60 + 14) * 60_000, now), 'in 2h 14m');
  assert.equal(until(now + 3 * 86_400_000 + 4 * 3_600_000, now), 'in 3d 4h');
  assert.equal(until(now - 1, now), 'now');
  assert.equal(ago(now - 3 * 60_000, now), '3 min ago');
  assert.equal(ago(now - 5 * 86_400_000, now), '5 d ago');
});

test('window names', () => {
  assert.equal(windowName(300), '5-hour');
  assert.equal(windowName(10080), 'Weekly');
  assert.equal(windowName(1440), '1-day');
});
