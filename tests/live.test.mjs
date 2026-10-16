import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(
  new URL('../src/island/live/liveActivity.ts', import.meta.url),
  'utf8',
);
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
});
const { pickLive, formatElapsed, formatRate } = await import(
  `data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`
);

const battery = (percent, plugged_in = false) => ({
  percent,
  charging: plugged_in,
  plugged_in,
  saver: false,
});
const base = {
  activity: { battery: battery(80), downloads: [], sensors: [] },
  powerChangedAt: null,
  now: 100_000,
  timerActive: false,
  mediaPlaying: false,
  privateMode: false,
};

test('nothing happening shows nothing', () => {
  assert.equal(pickLive(base), null);
  assert.equal(pickLive({ ...base, activity: null }), null);
});
test('privacy indicators win, and a meeting app reads as a call', () => {
  const live = pickLive({
    ...base,
    activity: {
      ...base.activity,
      downloads: [{ name: 'a.zip', bytes: 10, bytes_per_second: 5 }],
      sensors: [
        { kind: 'microphone', app: 'Zoom', since: 40_000 },
        { kind: 'camera', app: 'Zoom', since: 40_000 },
      ],
    },
  });
  assert.equal(live.kind, 'camera');
  const mic = pickLive({
    ...base,
    activity: { ...base.activity, sensors: [{ kind: 'microphone', app: 'Zoom', since: 1 }] },
  });
  assert.equal(mic.kind, 'call');
  const other = pickLive({
    ...base,
    activity: { ...base.activity, sensors: [{ kind: 'microphone', app: 'Chrome', since: 1 }] },
  });
  assert.equal(other.kind, 'microphone');
});
test('downloads add up', () => {
  const live = pickLive({
    ...base,
    activity: {
      ...base.activity,
      downloads: [
        { name: 'a.zip', bytes: 10, bytes_per_second: 5 },
        { name: 'b.iso', bytes: 30, bytes_per_second: 7 },
      ],
    },
  });
  assert.deepEqual(live, { kind: 'download', name: 'a.zip', count: 2, bytes: 40, rate: 12 });
});
test('plugging in shows briefly, then goes away', () => {
  const input = {
    ...base,
    activity: { ...base.activity, battery: battery(54, true) },
    powerChangedAt: 99_000,
  };
  assert.deepEqual(pickLive(input), { kind: 'power', percent: 54, pluggedIn: true });
  assert.equal(pickLive({ ...input, now: 110_000 }), null);
});
test('low battery waits behind the timer and music unless critical', () => {
  const low = { ...base, activity: { ...base.activity, battery: battery(18) } };
  assert.equal(pickLive(low).kind, 'battery-low');
  assert.equal(pickLive({ ...low, timerActive: true }), null);
  assert.equal(pickLive({ ...low, mediaPlaying: true }), null);
  const critical = {
    ...low,
    mediaPlaying: true,
    activity: { ...low.activity, battery: battery(8) },
  };
  assert.deepEqual(pickLive(critical), { kind: 'battery-low', percent: 8, critical: true });
  assert.equal(
    pickLive({ ...low, activity: { ...low.activity, battery: battery(18, true) } }),
    null,
  );
});
test('presentation mode hides every live activity', () => {
  const live = {
    ...base,
    privateMode: true,
    activity: { ...base.activity, sensors: [{ kind: 'screen', app: 'OBS', since: 1 }] },
  };
  assert.equal(pickLive(live), null);
});
test('formatting', () => {
  assert.equal(formatElapsed(40_000, 100_000), '1:00');
  assert.equal(formatElapsed(0, 3_725_000), '1:02:05');
  assert.equal(formatElapsed(null, 1), '');
  assert.equal(formatRate(2_621_440), '2.5 MB/s');
  assert.equal(formatRate(4096), '4 KB/s');
  assert.equal(formatRate(10), '');
});
