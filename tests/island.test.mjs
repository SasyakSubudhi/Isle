import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/app/islandReducer.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
});
const { initialIsland, reduceIsland: reduce } = await import(
  `data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`
);

test('explicit opening selects a tool and closing clears private previews and pinning', () => {
  const open = reduce(initialIsland, { type: 'open', tab: 'dictionary', pin: true });
  assert.equal(open.mode, 'expanded');
  assert.equal(open.tab, 'dictionary');
  assert.equal(open.pinned, true);
  const notice = reduce(open, { type: 'peek', notice: { title: 'Private copy', tab: 'shelf' } });
  assert.equal(notice.mode, 'expanded');
  const closed = reduce(notice, { type: 'close' });
  assert.equal(closed.notice, null);
  assert.equal(closed.pinned, false);
  assert.equal(closed.mode, 'idle');
});
test('incoming drag opens Shelf without overwriting persistent panel pin', () => {
  const state = reduce(initialIsland, { type: 'open', tab: 'media', pin: true });
  const dragging = reduce(state, { type: 'drag', active: true });
  assert.equal(dragging.tab, 'shelf');
  assert.equal(dragging.mode, 'expanded');
  assert.equal(dragging.pinned, true);
  assert.equal(reduce(dragging, { type: 'drag', active: false }).dragging, false);
});
test('switching tools preserves an explicitly pinned workspace', () => {
  const pinned = reduce(initialIsland, { type: 'open', tab: 'shelf', pin: true });
  const dictionary = reduce(pinned, { type: 'open', tab: 'dictionary' });
  assert.equal(dictionary.tab, 'dictionary');
  assert.equal(dictionary.pinned, true);
  const media = reduce(dictionary, { type: 'open', tab: 'media' });
  assert.equal(media.tab, 'media');
  assert.equal(media.pinned, true);
});
test('transient feedback opens a peek; toggle opens and closes the pinned workspace', () => {
  const peek = reduce(initialIsland, {
    type: 'peek',
    notice: { title: 'Timer done', tab: 'focus' },
  });
  assert.equal(peek.mode, 'peek');
  const open = reduce(peek, { type: 'toggle' });
  assert.equal(open.mode, 'expanded');
  assert.equal(open.pinned, true);
  assert.equal(reduce(open, { type: 'toggle' }).mode, 'idle');
});
