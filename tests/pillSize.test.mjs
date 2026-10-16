import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/island/pillSize.ts', import.meta.url), 'utf8');
const js = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { pillSize, PILL_MEDIUM, PILL_WIDE } = await import(
  `data:text/javascript;base64,${Buffer.from(js).toString('base64')}`
);
const idle = {
  expanded: false,
  dropping: false,
  dock: false,
  peek: false,
  live: false,
  mediaPlaying: false,
};

test('collapsed pill uses only three widths', () => {
  const widths = new Set(
    [
      idle,
      { ...idle, mediaPlaying: true },
      { ...idle, live: true },
      { ...idle, dropping: true },
      { ...idle, peek: true },
      { ...idle, dock: true },
    ].map((s) => pillSize(s, 148).width),
  );
  assert.deepEqual(
    [...widths].sort((a, b) => a - b),
    [148, PILL_MEDIUM, PILL_WIDE],
  );
});

test('a wider taskbar slot is never shrunk', () => {
  assert.equal(pillSize({ ...idle, mediaPlaying: true }, 260).width, 260);
  assert.equal(pillSize(idle, 260).width, 260);
});

test('expanded panel keeps its size', () => {
  assert.deepEqual(pillSize({ ...idle, expanded: true, peek: true }, 148), {
    width: 432,
    height: 324,
  });
});
