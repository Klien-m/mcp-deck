import { readFileSync, writeFileSync } from 'node:fs';

const tag = process.argv[2] ?? process.env.GITHUB_REF_NAME;
const match = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)(?:\.(0|[1-9]\d*))?$/.exec(tag || '');
if (!match || match[0] !== tag) {
  throw new Error('发布标签必须为 v主版本.次版本 或 v主版本.次版本.修订号（v 可省略），例如 v0.2、v0.2.1');
}
const version = `${match[1]}.${match[2]}.${match[3] || '0'}`;

// 只替换应用版本字段，保留文件格式、依赖版本和 DMG 定制配置。
const rootJsonVersion = /^(\s*"version"\s*:\s*")[^"]+("[^\r\n]*)/m;
const files = [
  ['package.json', [rootJsonVersion]],
  ['package-lock.json', [rootJsonVersion, /(""\s*:\s*\{[^}]*?"version"\s*:\s*")[^"]+(")/]],
  ['src-tauri/tauri.conf.json', [rootJsonVersion]],
  ['src-tauri/Cargo.toml', [/(^\[package\]\r?\n(?:(?!\[)[^\n]*\n)*?version\s*=\s*")[^"]+(")/m]],
  ['src-tauri/Cargo.lock', [/(^\[\[package\]\]\r?\nname = "mcp-deck"\r?\nversion = ")[^"]+(")/m]],
];

// 先检查所有文件，再写入，避免配置缺失时留下部分更新的版本。
const updates = files.map(([path, patterns]) => {
  let contents = readFileSync(path, 'utf8');
  for (const pattern of patterns) {
    if (!pattern.test(contents)) {
      throw new Error(`${path} 中未找到应用版本字段`);
    }
    contents = contents.replace(pattern, (_, prefix, suffix) => `${prefix}${version}${suffix}`);
  }
  return [path, contents];
});
for (const [path, contents] of updates) {
  writeFileSync(path, contents);
}
console.log(`发布版本：${tag} → ${version}`);
