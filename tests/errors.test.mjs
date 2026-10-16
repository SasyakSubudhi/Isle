import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/lib/errors.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
});
const { errorMessage } = await import(
  `data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`
);

test('error states show a clean sentence', () => {
  assert.equal(errorMessage('Usage database is busy'), 'Usage database is busy');
  assert.equal(errorMessage(new Error('Network down')), 'Network down');
  assert.equal(errorMessage('Error: Settings busy'), 'Settings busy');
  assert.equal(errorMessage(''), 'Something went wrong.');
});
