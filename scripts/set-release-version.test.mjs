import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const script = fileURLToPath(new URL('./set-release-version.mjs', import.meta.url));
const fixtures = {
  'package.json': '{\n  "name": "mcp-deck",\n  "version": "0.1.0",\n  "dependencies": {"example": "0.1.0"}\n}\n',
  'package-lock.json': '{\n  "name": "mcp-deck",\n  "version": "0.1.0",\n  "lockfileVersion": 3,\n  "packages": {\n    "": {\n      "version": "0.1.0",\n      "dependencies": {"example": "0.1.0"}\n    },\n    "node_modules/example": {"version": "0.1.0"}\n  }\n}\n',
  'src-tauri/tauri.conf.json': '{\n  "productName": "MCP Deck",\n  "version": "0.1.0",\n  "bundle": {"macOS":{"dmg":{"background":"dmg/background.png"}}}\n}\n',
  'src-tauri/Cargo.toml': '[package]\nname = "mcp-deck"\nversion = "0.1.0"\n\n[dependencies]\nexample = { version = "0.1.0" }\n',
  'src-tauri/Cargo.lock': 'version = 4\n\n[[package]]\nname = "example"\nversion = "0.1.0"\n\n[[package]]\nname = "mcp-deck"\nversion = "0.1.0"\ndependencies = ["example"]\n\n[[package]]\nname = "other"\nversion = "0.1.0"\n',
};

function workspace(t) {
  const directory = mkdtempSync(join(tmpdir(), 'mcp-deck-release-version-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  mkdirSync(join(directory, 'src-tauri'));
  for (const [path, contents] of Object.entries(fixtures)) {
    writeFileSync(join(directory, path), contents);
  }
  return {
    directory,
    read: (path) => readFileSync(join(directory, path), 'utf8'),
    run: (tag) => spawnSync(process.execPath, [script], {
      cwd: directory,
      encoding: 'utf8',
      env: { ...process.env, GITHUB_REF_NAME: tag },
    }),
  };
}

for (const [tag, version] of [['v0.2', '0.2.0'], ['v0.2.1', '0.2.1'], ['1.12', '1.12.0'], ['1.12.3', '1.12.3'], ['v0.1.0', '0.1.0']]) {
  test(`${tag} updates all application versions without changing dependencies or formatting`, (t) => {
    const { read, run } = workspace(t);
    const result = run(tag);
    assert.equal(result.status, 0, result.stderr);
    for (const path of ['package.json', 'src-tauri/tauri.conf.json']) {
      assert.equal(read(path), fixtures[path].replace('"version": "0.1.0"', `"version": "${version}"`));
    }
    const lock = JSON.parse(read('package-lock.json'));
    assert.equal(lock.version, version);
    assert.equal(lock.packages[''].version, version);
    assert.equal(lock.packages[''].dependencies.example, '0.1.0');
    assert.equal(lock.packages['node_modules/example'].version, '0.1.0');
    assert.equal(read('package-lock.json'), fixtures['package-lock.json'].replace(/"version": "0.1.0"/g, (value, offset) => offset < fixtures['package-lock.json'].indexOf('"dependencies"') ? `"version": "${version}"` : value));
    assert.equal(read('src-tauri/Cargo.toml'), fixtures['src-tauri/Cargo.toml'].replace('version = "0.1.0"', `version = "${version}"`));
    assert.equal(read('src-tauri/Cargo.lock'), fixtures['src-tauri/Cargo.lock'].replace('name = "mcp-deck"\nversion = "0.1.0"', `name = "mcp-deck"\nversion = "${version}"`));
    const beforeRerun = Object.fromEntries(Object.keys(fixtures).map((path) => [path, read(path)]));
    assert.equal(run(tag).status, 0);
    for (const [path, contents] of Object.entries(beforeRerun)) assert.equal(read(path), contents);
  });
}

test('invalid tags fail before writing files', (t) => {
  const { read, run } = workspace(t);
  for (const tag of ['', 'latest', 'release/v0.2', 'v1', 'v01.2', 'v1.02.3', 'v1.2.03', 'v1.2.3.4', 'v1.2.3-beta.1', 'v1.2.3+build', 'v1.2;echo unsafe', 'v0.2\n', ' v0.2']) {
    const result = run(tag);
    assert.notEqual(result.status, 0, tag);
    assert.match(result.stderr, /发布标签必须为/);
    for (const [path, contents] of Object.entries(fixtures)) assert.equal(read(path), contents);
  }
});

test('a missing Cargo lock entry fails before any versions are written', (t) => {
  const { directory, read, run } = workspace(t);
  writeFileSync(join(directory, 'src-tauri/Cargo.lock'), 'version = 4\n');
  assert.notEqual(run('v0.2').status, 0);
  for (const path of Object.keys(fixtures).filter((path) => path !== 'src-tauri/Cargo.lock')) {
    assert.equal(read(path), fixtures[path]);
  }
});

test('Windows CRLF files keep their line endings', (t) => {
  const { directory, read, run } = workspace(t);
  for (const [path, contents] of Object.entries(fixtures)) writeFileSync(join(directory, path), contents.replaceAll('\n', '\r\n'));
  const result = run('v0.2');
  assert.equal(result.status, 0, result.stderr);
  for (const path of Object.keys(fixtures)) assert.doesNotMatch(read(path), /(?<!\r)\n/);
});

test('a requested tag overrides the workflow branch during a rebuild', (t) => {
  const { directory, read } = workspace(t);
  const result = spawnSync(process.execPath, [script, 'v0.3'], {
    cwd: directory,
    encoding: 'utf8',
    env: { ...process.env, GITHUB_REF_NAME: 'master' },
  });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(read('src-tauri/tauri.conf.json')).version, '0.3.0');
});
