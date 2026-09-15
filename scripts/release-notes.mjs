import { execFileSync } from 'node:child_process';

const tag = process.argv[2] ?? process.env.GITHUB_REF_NAME;
const repository = process.env.GITHUB_REPOSITORY;
if (!tag || !repository) {
  throw new Error('A release tag and GITHUB_REPOSITORY are required');
}

const git = (...args) => execFileSync('git', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
const commit = git('rev-parse', 'HEAD');
const url = `${process.env.GITHUB_SERVER_URL || 'https://github.com'}/${repository}`;
let previousTag;
try {
  previousTag = git('describe', '--tags', '--abbrev=0', `${commit}^`);
} catch {
  // The first release includes the complete history.
}

const range = previousTag ? `refs/tags/${previousTag}..${commit}` : commit;
const changes = git('log', '--no-merges', '--format=%H %s', range).split('\n').filter(Boolean).map((line) => {
  const hash = line.slice(0, 40);
  return `- ${line.slice(41)} ([${hash.slice(0, 7)}](${url}/commit/${hash}))`;
});

console.log(`## 更新说明

${previousTag ? `自 ${previousTag} 以来的更新：` : '首次发布，包含以下更新：'}

${changes.join('\n') || '- 本次没有新的非合并提交。'}

${previousTag ? `[完整变更](${url}/compare/${encodeURIComponent(previousTag)}...${encodeURIComponent(tag)})` : `[提交记录](${url}/commits/${encodeURIComponent(tag)})`}

## 下载

| 平台 | 架构 | 产物 |
| --- | --- | --- |
| Windows | x64 | NSIS 安装程序（.exe）、MSI 安装包（.msi） |
| macOS | Apple Silicon（aarch64）、Intel（x64） | 各一份 .dmg |
| Linux | x64 | .deb、.rpm、.AppImage |

macOS 使用 ad hoc 签名，尚未进行 Apple 公证；Windows 安装包未进行代码签名。系统可能提示确认应用来源。
`);
