import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/lib/settingsDraft.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { changedFields, rebaseDraft, clampNumber, hotkeyFromEvent } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);

test('save sends only the fields the user edited', () => {
  const base = { theme: 'dark', tracking_paused: false, hotkeys: { toggle: 'Ctrl+Alt+Space' } };
  const draft = { ...base, theme: 'light' };
  assert.deepEqual(changedFields(base, draft), { theme: 'light' });
  const hotkeys = { ...base, hotkeys: { toggle: 'Ctrl+Alt+I' } };
  assert.deepEqual(changedFields(base, hotkeys), { hotkeys: { toggle: 'Ctrl+Alt+I' } });
});

test('a tray change reaches the open form without losing unsaved edits', () => {
  const base = { theme: 'dark', tracking_paused: false };
  const draft = { ...base, theme: 'light' };
  const incoming = { theme: 'dark', tracking_paused: true };
  assert.deepEqual(rebaseDraft(base, draft, incoming), { theme: 'light', tracking_paused: true });
});

test('number fields stay inside the accepted range', () => {
  assert.equal(clampNumber('', 1, 180, 25), 25);
  assert.equal(clampNumber('abc', 1, 180, 25), 25);
  assert.equal(clampNumber('0', 1, 180, 25), 1);
  assert.equal(clampNumber('999', 1, 180, 25), 180);
  assert.equal(clampNumber('42.6', 1, 180, 25), 43);
});

test('hotkey recorder produces plugin-style combos', () => {
  const press = (key, code, mods = {}) => ({ key, code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods });
  assert.equal(hotkeyFromEvent(press('i', 'KeyI', { ctrlKey: true, altKey: true })), 'Ctrl+Alt+I');
  assert.equal(hotkeyFromEvent(press(' ', 'Space', { ctrlKey: true, altKey: true })), 'Ctrl+Alt+Space');
  assert.equal(hotkeyFromEvent(press('F5', 'F5', { shiftKey: true })), 'Shift+F5');
  assert.equal(hotkeyFromEvent(press('!', 'Digit1', { ctrlKey: true, shiftKey: true })), 'Ctrl+Shift+1');
  assert.equal(hotkeyFromEvent(press('Control', 'ControlLeft', { ctrlKey: true })), null);
  assert.equal(hotkeyFromEvent(press('d', 'KeyD')), null);
});
