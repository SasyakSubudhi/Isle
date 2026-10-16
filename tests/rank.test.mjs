import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const load = async (path) => {
  const source = await readFile(new URL(path, import.meta.url), 'utf8');
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  });
  return import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
};
const { rank, score } = await load('../src/lib/rank.ts');
const { dndActive, inCall } = await load('../src/lib/dnd.ts');

test('palette ranks prefixes and word starts first', () => {
  const items = ['Scratchpad', 'Screen time', 'Settings', 'Start focus timer'];
  assert.equal(rank(items, 'set', (x) => x)[0], 'Settings');
  assert.equal(rank(items, 'st', (x) => x)[0], 'Start focus timer');
  assert.equal(rank(items, 'sft', (x) => x)[0], 'Start focus timer');
  assert.equal(score('zz', 'Settings'), -1);
  assert.deepEqual(rank(items, '', (x) => x), items);
});

test('Do Not Disturb is manual or automatic during calls', () => {
  const call = { sensors: [{ kind: 'microphone' }] };
  assert.equal(inCall(call), true);
  assert.equal(inCall({ sensors: [] }), false);
  assert.equal(dndActive({ do_not_disturb: true, auto_dnd: false }, null), true);
  assert.equal(dndActive({ do_not_disturb: false, auto_dnd: true }, call), true);
  assert.equal(dndActive({ do_not_disturb: false, auto_dnd: false }, call), false);
  assert.equal(dndActive(undefined, call), false);
});
