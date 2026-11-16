#!/usr/bin/env node
// Generates the release-dependent parts of the static site from CHANGELOG.md:
// the release list in site/changelog.html and the latest version and date
// wherever a page has <!-- clipsa:version --> / <!-- clipsa:date --> markers.
// Only dated sections count, so an "Unreleased" section never shows up.
//
//   node scripts/build-site.mjs           rewrite the files
//   node scripts/build-site.mjs --check   exit 1 if any file is out of date
//
// No dependencies; the site has no build step and serves these files as is.
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = new URL('..', import.meta.url).pathname;
const SITE = join(ROOT, 'site');
const REPO = 'https://github.com/mordamax/clipsa';
const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];

function releases(changelog) {
  const out = [];
  let current = null;
  for (const line of changelog.split('\n')) {
    const heading = line.match(/^## \[([^\]]+)\] - (.+)$/);
    if (heading) {
      const [, version, date] = heading;
      current = /^\d{4}-\d{2}-\d{2}$/.test(date.trim()) ? { version, date: date.trim(), lines: [] } : null;
      if (current) out.push(current);
    } else if (line.startsWith('## ')) {
      current = null;
    } else if (current) {
      current.lines.push(line);
    }
  }
  if (out.length === 0) throw new Error('CHANGELOG.md has no dated release section');
  return out;
}

function longDate(iso) {
  const [y, m, d] = iso.split('-').map(Number);
  return `${d} ${MONTHS[m - 1]} ${y}`;
}

const escape = (text) => text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

function inline(text) {
  // Odd segments between backticks are code spans.
  return text
    .split('`')
    .map((part, i) => {
      if (i % 2 === 1) return `<code>${escape(part)}</code>`;
      return escape(part)
        .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
        .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, label, url) => `<a href="${url.startsWith('./') ? `${REPO}/blob/master/${url.slice(2)}` : url}">${label}</a>`);
    })
    .join('');
}

// The Markdown that CHANGELOG.md uses: ### headings, "- " bullets (indented
// lines continue a bullet), paragraphs, **bold**, `code` and links.
function renderBody(lines, indent) {
  const html = [];
  let list = null;
  let paragraph = null;
  const flush = () => {
    if (paragraph) html.push(`<p>${inline(paragraph.join(' '))}</p>`);
    if (list) html.push('<ul>', ...list.map((item) => `  <li>${inline(item)}</li>`), '</ul>');
    paragraph = list = null;
  };
  for (const raw of lines) {
    const line = raw.trimEnd();
    if (!line.trim()) {
      flush();
    } else if (line.startsWith('### ')) {
      flush();
      html.push(`<h3>${inline(line.slice(4))}</h3>`);
    } else if (line.startsWith('- ')) {
      if (paragraph) flush();
      (list ??= []).push(line.slice(2));
    } else if (list && /^\s/.test(line)) {
      list[list.length - 1] += ` ${line.trim()}`;
    } else {
      if (list) flush();
      (paragraph ??= []).push(line.trim());
    }
  }
  flush();
  return html.map((l) => indent + l).join('\n');
}

function renderReleases(all) {
  const pad = '      ';
  return all
    .map(({ version, date, lines }) =>
      [
        `${pad}<section class="release" id="v${version}" aria-labelledby="v${version}-title">`,
        `${pad}  <h2 id="v${version}-title">Clipsa ${version}</h2>`,
        `${pad}  <p class="meta"><time datetime="${date}">${longDate(date)}</time> · <a href="${REPO}/releases/tag/v${version}">Release on GitHub</a></p>`,
        renderBody(lines, `${pad}  `),
        `${pad}</section>`
      ].join('\n')
    )
    .join('\n\n');
}

function renderToc(all) {
  return all.map(({ version }) => `        <li><a href="#v${version}">Clipsa ${version}</a></li>`).join('\n');
}

/** Replace what lies between <!-- clipsa:NAME --> and <!-- /clipsa:NAME -->. */
function fill(html, name, content, block) {
  const pattern = new RegExp(`(<!-- clipsa:${name} -->)[\\s\\S]*?(\\s*<!-- /clipsa:${name} -->)`, 'g');
  return html.replace(pattern, (_, open, close) => (block ? `${open}\n${content}${close}` : `${open}${content}<!-- /clipsa:${name} -->`));
}

const all = releases(readFileSync(join(ROOT, 'CHANGELOG.md'), 'utf8'));
const latest = all[0];
const check = process.argv.includes('--check');
const stale = [];

for (const file of readdirSync(SITE).filter((f) => f.endsWith('.html'))) {
  const path = join(SITE, file);
  const before = readFileSync(path, 'utf8');
  let after = fill(before, 'version', latest.version, false);
  after = fill(after, 'date', `<time datetime="${latest.date}">${longDate(latest.date)}</time>`, false);
  after = fill(after, 'releases', renderReleases(all), true);
  after = fill(after, 'toc', renderToc(all), true);
  if (after === before) continue;
  stale.push(`site/${file}`);
  if (!check) writeFileSync(path, after);
}

if (check && stale.length) {
  console.error(`Out of date with CHANGELOG.md (latest ${latest.version}): ${stale.join(', ')}. Run: pnpm site`);
  process.exit(1);
}
console.log(stale.length ? `Updated ${stale.join(', ')} (latest ${latest.version})` : `Site is up to date (latest ${latest.version})`);
