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
test('incoming drag switches an open panel to Shelf without overwriting its pin', () => {
  const state = reduce(initialIsland, { type: 'open', tab: 'media', pin: true });
  const hovering = reduce(state, { type: 'dropHover', active: true });
  assert.equal(hovering.tab, 'shelf');
  assert.equal(hovering.mode, 'expanded');
  assert.equal(hovering.pinned, true);
  assert.equal(reduce(hovering, { type: 'dropHover', active: false }).dropHover, false);
});
test('incoming drag over the collapsed pill reacts in place instead of opening the panel', () => {
  const hovering = reduce(initialIsland, { type: 'dropHover', active: true });
  assert.equal(hovering.mode, 'idle');
  assert.equal(hovering.dropHover, true);
  assert.equal(reduce(hovering, { type: 'dropHover', active: true }), hovering);
});
test('dragging out keeps the current view', () => {
  const dock = reduce(initialIsland, { type: 'dock' });
  const dragging = reduce(dock, { type: 'drag', active: true });
  assert.equal(dragging.mode, 'dock');
  assert.equal(dragging.dragging, true);
});
test('dock tray opens from the pill, survives peeks and never replaces the panel', () => {
  const dock = reduce(initialIsland, { type: 'dock' });
  assert.equal(dock.mode, 'dock');
  const peeked = reduce(dock, { type: 'peek', notice: { title: 'a.txt added', tab: 'shelf' } });
  assert.equal(peeked.mode, 'dock');
  const open = reduce(initialIsland, { type: 'open', tab: 'notes' });
  assert.equal(reduce(open, { type: 'dock' }), open);
  assert.equal(reduce(dock, { type: 'open', tab: 'shelf' }).mode, 'expanded');
  assert.equal(reduce(dock, { type: 'close' }).mode, 'idle');
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
