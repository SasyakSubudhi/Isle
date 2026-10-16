import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/lib/clipSmart.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
});
const { detectClip, parseColor, toHex, toRgb, toHsl, cleanLink } = await import(
  `data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`
);

const kind = (text) => detectClip(text).kind;
const action = (text, id) => detectClip(text).actions.find((a) => a.id === id);

test('links get open, clean and Markdown actions', () => {
  const info = detectClip('https://www.example.com/a?id=4&utm_source=mail&fbclid=x');
  assert.equal(info.kind, 'link');
  assert.equal(info.label, 'example.com');
  assert.equal(info.actions[0].type, 'open');
  assert.equal(action('https://example.com/?utm_medium=x', 'clean').value, 'https://example.com/');
  assert.equal(
    action('https://www.example.com/a', 'markdown').value,
    '[example.com](https://www.example.com/a)',
  );
  assert.equal(kind('www.example.com/path'), 'link');
  assert.equal(action('www.example.com', 'open').value, 'https://www.example.com/');
  assert.equal(action('https://example.com/a', 'clean'), undefined);
});

test('colors convert between HEX, RGB and HSL', () => {
  assert.deepEqual(parseColor('#fff'), { r: 255, g: 255, b: 255, a: 1 });
  assert.deepEqual(parseColor('rgb(10, 20, 30)'), { r: 10, g: 20, b: 30, a: 1 });
  assert.deepEqual(parseColor('rgb(10 20 30 / 50%)'), { r: 10, g: 20, b: 30, a: 0.5 });
  assert.equal(toHex(parseColor('hsl(210, 100%, 77%)')), '#8AC4FF');
  assert.equal(toHsl(parseColor('#8ab4ff')), 'hsl(218, 100%, 77%)');
  assert.equal(toRgb(parseColor('#8ab4ff80')), 'rgba(138, 180, 255, 0.502)');
  const info = detectClip('#8ab4ff');
  assert.equal(info.kind, 'color');
  assert.equal(info.label, '#8AB4FF');
  // The format already copied isn't offered again.
  assert.deepEqual(
    info.actions.map((a) => a.id),
    ['rgb', 'hsl'],
  );
  assert.equal(action('rgb(138, 180, 255)', 'hex').value, '#8AB4FF');
});

test('words and numbers are not mistaken for colors', () => {
  assert.equal(parseColor('add'), null);
  assert.equal(parseColor('1234'), null);
  assert.equal(parseColor('decade'), null);
  assert.equal(parseColor('decade1'), null);
  assert.notEqual(parseColor('8ab4ff'), null);
  assert.equal(parseColor('rgb(1, 2)'), null);
});

test('one-time codes are found alone and inside messages', () => {
  assert.equal(action('482193', 'copy-code').value, '482193');
  assert.equal(action('482 193', 'copy-code').value, '482193');
  assert.equal(
    action('Your verification code is 482193. Do not share it.', 'copy-code').value,
    '482193',
  );
  assert.equal(action('G-123456 is your Google verification code', 'copy-code').value, '123456');
  assert.equal(kind('Meet me at 10:30 on 2026-10-16'), 'text');
  assert.equal(kind('Order total 1299.50, code is not needed'), 'text');
});

test('emails and phone numbers', () => {
  const email = detectClip('mailto:Sam@Example.co.uk');
  assert.equal(email.kind, 'email');
  assert.equal(action('sam@example.com', 'mail').value, 'mailto:sam@example.com');
  assert.equal(action('sam@example.com', 'domain').value, 'example.com');
  assert.equal(kind('+1 (555) 123-4567'), 'phone');
  assert.equal(action('+1 (555) 123-4567', 'call').value, 'tel:+15551234567');
  assert.equal(action('98765 43210', 'copy-digits').value, '9876543210');
  assert.equal(kind('2026-10-16'), 'text');
  assert.equal(kind('12345678'), 'otp');
});

test('addresses open in Maps and flatten to one line', () => {
  const text = '1600 Amphitheatre Parkway\nMountain View, CA 94043';
  assert.equal(kind(text), 'address');
  assert.equal(
    action(text, 'one-line').value,
    '1600 Amphitheatre Parkway, Mountain View, CA 94043',
  );
  assert.match(
    action(text, 'map').value,
    /^https:\/\/www\.google\.com\/maps\/search\/\?api=1&query=1600/,
  );
  assert.equal(kind('221B Baker Street, London NW1 6XE'), 'address');
  assert.equal(kind('I walked down the road today'), 'text');
});

test('code gets a Markdown block and JSON gets format actions', () => {
  // Shelf stores text trimmed, as the Rust side saves it.
  const js = 'const total = items.reduce((a, b) => a + b, 0);\nreturn total;';
  const info = detectClip(js);
  assert.equal(info.kind, 'code');
  assert.equal(info.label, 'Code · js');
  assert.equal(action(js, 'markdown').value, '```js\n' + js + '\n```');
  assert.equal(kind('def add(a, b):\n    return a + b'), 'code');
  assert.equal(detectClip('def add(a, b):\n    return a + b').label, 'Code · python');
  assert.equal(action('{"a":1,"b":[2]}', 'pretty').value, '{\n  "a": 1,\n  "b": [\n    2\n  ]\n}');
  assert.equal(action('{\n  "a": 1\n}', 'minify').value, '{"a":1}');
  assert.equal(kind('SELECT id, name FROM users WHERE id = 4;'), 'code');
});

test('prose stays plain text but offers what is inside it', () => {
  const prose =
    'Thanks for the call today. The notes are at https://example.com/notes?utm_source=x. Talk soon!';
  const info = detectClip(prose);
  assert.equal(info.kind, 'text');
  assert.deepEqual(
    info.actions.map((a) => a.id),
    ['open', 'copy-link', 'save-link', 'clean', 'markdown'],
  );
  assert.equal(info.actions[1].value, 'https://example.com/notes?utm_source=x');
  assert.equal(kind('Just a normal sentence, nothing special here.'), 'text');
  assert.deepEqual(detectClip('').actions, []);
  assert.deepEqual(detectClip(null).actions, []);
});

test('helpers', () => {
  assert.equal(cleanLink('not a url'), 'not a url');
});
