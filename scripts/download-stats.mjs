#!/usr/bin/env node
// Download counts per release from GitHub Releases, which serve every Clipsa
// download: the website's buttons, direct links, Homebrew (the cask fetches the
// DMG) and the in-app updater. Read-only; nothing is tracked beyond what GitHub
// already counts.
//
//   pnpm stats            all published releases
//   pnpm stats --json     machine-readable
//
// Uses GITHUB_TOKEN (or `gh auth token`) when available for a higher API rate
// limit; works without one. Counts include bots that mirror every file and the
// downloads the release workflow makes itself (the Homebrew job reads the DMGs).
import { execFileSync } from 'node:child_process';

const REPO = 'mordamax/clipsa';

// Column → file names. Windows and Linux updates download the same installer
// files, so those columns mix new installs and updates; macOS updates use the
// .app.tar.gz bundles, which are counted separately.
const COLUMNS = [
  ['macOS', (n) => n.endsWith('.dmg')],
  ['Windows', (n) => n.endsWith('.msi') || n.endsWith('-setup.exe')],
  ['Linux', (n) => /\.(AppImage|deb|rpm)$/.test(n)],
  ['macOS updates', (n) => n.endsWith('.app.tar.gz')],
  ['update checks', (n) => n === 'latest.json']
];

function token() {
  if (process.env.GITHUB_TOKEN) return process.env.GITHUB_TOKEN;
  try {
    return execFileSync('gh', ['auth', 'token'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
  } catch {
    return '';
  }
}

async function releases() {
  const auth = token();
  const all = [];
  for (let page = 1; ; page++) {
    const response = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=100&page=${page}`, {
      headers: { Accept: 'application/vnd.github+json', ...(auth ? { Authorization: `Bearer ${auth}` } : {}) }
    });
    if (!response.ok) throw new Error(`GitHub API ${response.status}: ${await response.text()}`);
    const batch = await response.json();
    all.push(...batch.filter((r) => !r.draft));
    if (batch.length < 100) return all;
  }
}

const rows = (await releases()).map((release) => {
  const counts = Object.fromEntries(COLUMNS.map(([name]) => [name, 0]));
  for (const asset of release.assets) {
    const column = COLUMNS.find(([, matches]) => matches(asset.name));
    if (column) counts[column[0]] += asset.download_count;
  }
  return { version: release.tag_name, published: release.published_at.slice(0, 10), ...counts };
});

if (process.argv.includes('--json')) {
  console.log(JSON.stringify(rows, null, 2));
} else {
  const total = { version: 'total', published: '' };
  for (const [name] of COLUMNS) total[name] = rows.reduce((sum, row) => sum + row[name], 0);
  const header = ['version', 'published', ...COLUMNS.map(([name]) => name)];
  const table = [header, ...[...rows, total].map((row) => header.map((key) => String(row[key])))];
  const widths = header.map((_, i) => Math.max(...table.map((cells) => cells[i].length)));
  const line = (cells) => cells.map((cell, i) => (i < 2 ? cell.padEnd(widths[i]) : cell.padStart(widths[i]))).join('  ');
  console.log(line(table[0]));
  console.log(widths.map((w) => '-'.repeat(w)).join('  '));
  table.slice(1, -1).forEach((cells) => console.log(line(cells)));
  console.log(widths.map((w) => '-'.repeat(w)).join('  '));
  console.log(line(table.at(-1)));
  console.log('\nInstallers count new installs (Windows/Linux also updates); "macOS updates" are in-app updates installed;');
  console.log('"update checks" are fetches of that release\'s latest.json. Bots and the release workflow add some noise.');
}
