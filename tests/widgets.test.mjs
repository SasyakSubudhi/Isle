import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

async function load(path) {
  const source = await readFile(new URL(path, import.meta.url), 'utf8');
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  });
  return import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
}
const widgets = await load('../src/modules/widgets/widgetLogic.ts');
const context = await load('../src/modules/context/contextLabel.ts');

test('habit days use the local calendar and toggle cleanly', () => {
  const today = new Date(2026, 9, 16, 23, 30);
  assert.equal(widgets.localDate(today), '2026-10-16');
  assert.deepEqual(widgets.lastDays(today, 3), ['2026-10-14', '2026-10-15', '2026-10-16']);
  assert.deepEqual(widgets.lastDays(new Date(2026, 2, 1), 2), ['2026-02-28', '2026-03-01']);
  const habit = widgets.newHabit('  Read  ', 'a');
  assert.equal(habit.name, 'Read');
  const once = widgets.toggleDay(habit, '2026-10-16');
  assert.deepEqual(once.done, ['2026-10-16']);
  assert.deepEqual(widgets.toggleDay(once, '2026-10-16').done, []);
});

test('streak counts back from today, or from yesterday before today is done', () => {
  const today = new Date(2026, 9, 16, 9);
  const habit = { id: 'a', name: 'Walk', done: ['2026-10-13', '2026-10-14', '2026-10-15'] };
  assert.equal(widgets.streak(habit, today), 3);
  assert.equal(widgets.streak({ ...habit, done: [...habit.done, '2026-10-16'] }, today), 4);
  assert.equal(widgets.streak({ ...habit, done: ['2026-10-12'] }, today), 0);
});

test('weather codes read as words', () => {
  assert.equal(widgets.weatherWords(0), 'Clear');
  assert.equal(widgets.weatherWords(63), 'Rain');
  assert.equal(widgets.weatherWords(81), 'Rain');
  assert.equal(widgets.weatherWords(73), 'Snow');
  assert.equal(widgets.weatherWords(96), 'Thunderstorm');
  assert.equal(widgets.weatherGlyph(0, false), '☾');
  assert.equal(widgets.degrees(21.6), '22°');
  assert.equal(widgets.degrees(null), '–');
});

test('context pill labels and elapsed time', () => {
  assert.equal(context.contextLabel('code'), 'Coding');
  assert.equal(context.formatElapsed(30_000), 'now');
  assert.equal(context.formatElapsed(12 * 60_000 + 5_000), '12m');
  assert.equal(context.formatElapsed(65 * 60_000), '1h 05m');
  assert.equal(context.formatMeeting(75_000), '01:15');
  assert.equal(context.formatMeeting(3_725_000), '1:02:05');
  assert.equal(context.tickMs('meeting'), 1000);
});
