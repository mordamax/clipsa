#!/usr/bin/env node
// Generates the release-dependent parts of the static site from CHANGELOG.md:
// the release list in site/changelog.html, the latest version and date
// wherever a page has <!-- clipsa:version --> / <!-- clipsa:date --> markers,
// and one page per installer in site/download/ (see DOWNLOADS).
// Only dated sections count, so an undated one never shows up.
//
//   node scripts/build-site.mjs           rewrite the files
//   node scripts/build-site.mjs --check   exit 1 if any file is out of date
//   node scripts/build-site.mjs --assets  print the installer file names the
//                                         site links to, one per line
//
// No dependencies; the site has no build step and serves these files as is.
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = new URL('..', import.meta.url).pathname;
const SITE = join(ROOT, 'site');
const REPO = 'https://github.com/mordamax/clipsa';
const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];

const kbd = (...keys) => keys.map((key) => `<kbd>${key}</kbd>`).join(' ');
const OPEN_PICKER = `The welcome screen asks whether Clipsa may check for updates. Then press ${kbd('Ctrl', 'Shift', 'V')} in any app to open the quick picker, and ${kbd('Enter')} to paste.`;

const macSteps = (file) => [
  ['Move it to Applications', `Open <code>${file}</code> in your Downloads and drag Clipsa onto the Applications folder.`],
  ['Start Clipsa', 'Open Clipsa from Applications or Spotlight. It is signed and notarized by Apple, so when macOS asks about an app downloaded from the internet, choose <strong>Open</strong>. Clipsa lives in the menu bar and keeps running when you close its window.'],
  ['Set up pasting', `On the welcome screen, <strong>Allow Accessibility…</strong> lets ${kbd('Enter')} paste into the app you were using (without it, you press ${kbd('⌘', 'V')} yourself), and you choose whether Clipsa may check for updates. Then press ${kbd('⌘', '⇧', 'V')} in any app.`]
];

const windowsSteps = (file) => [
  ['Run the installer', `Open <code>${file}</code> from your Downloads. Clipsa isn't code-signed for Windows yet, so SmartScreen may say it protected your PC: choose <strong>More info</strong>, then <strong>Run anyway</strong>.`],
  ['Start Clipsa', 'Open Clipsa from the Start menu. It lives in the system tray (look under the ^ arrow) and keeps running when you close its window.'],
  ['Open the picker', OPEN_PICKER]
];

const LINUX_START = 'Clipsa stores its key in the Secret Service, so GNOME Keyring, KWallet or another provider must be running. It keeps running in the tray when you close its window; on GNOME, tray icons need the AppIndicator extension, which Ubuntu includes.';
const LINUX_PICKER = `${OPEN_PICKER} Linux can't tell Clipsa which copies are secret, so use <strong>Pause monitoring</strong> in the tray before copying passwords.`;

const linuxPackageSteps = (install) => [
  ['Install the package', `From the folder you downloaded to, run:<pre><code>${install}</code></pre>`],
  ['Start Clipsa', `Open Clipsa from your applications menu. ${LINUX_START}`],
  ['Open the picker', LINUX_PICKER]
];

const appImageSteps = (file) => [
  ['Make it executable', `In the file's Properties, allow running it as a program, or run:<pre><code>chmod +x ${file}</code></pre>`],
  ['Start Clipsa', `Double-click <code>${file}</code>. ${LINUX_START}`],
  ['Open the picker', LINUX_PICKER]
];

const unixVerify = (sha256) => (file, sums) => ({
  code: `cd ~/Downloads\ncurl -fLO ${sums}\n${sha256} -c --ignore-missing SHA256SUMS`,
  note: `It prints <code>${file}: OK</code> when the file matches.`
});

const macVerify = (file, sums) => {
  const unix = unixVerify('shasum -a 256')(file, sums);
  return { ...unix, note: `${unix.note} <code>spctl -a -vv -t install ${file}</code> shows Apple's notarization: <code>source=Notarized Developer ID</code>.` };
};

const linuxVerify = unixVerify('sha256sum');

const MAC_HINT = 'Not sure which Mac you have? Apple menu → About This Mac lists a “Chip” on Apple Silicon and a “Processor” on Intel.';

const windowsVerify = (file, sums) => ({
  code: `cd ~\\Downloads\n$hash = (Get-FileHash .\\${file}).Hash.ToLower()\ncurl.exe -fsSL ${sums} | Select-String $hash`,
  note: `In PowerShell. It prints the line for <code>${file}</code> when the file matches, and nothing when it doesn't.`
});

// Every installer the website links to, each with a page at /download/<slug>
// that starts the download from GitHub Releases (a meta refresh: the site runs
// no scripts) and shows how to install and verify it. The release workflow
// checksums exactly these files into SHA256SUMS (--assets), so a renamed
// bundle fails the release instead of breaking a download link.
const DOWNLOADS = [
  {
    slug: 'macos',
    system: 'macOS',
    label: 'Apple Silicon',
    file: (v) => `Clipsa_${v}_aarch64.dmg`,
    hint: MAC_HINT,
    steps: macSteps,
    verify: macVerify
  },
  {
    slug: 'macos-intel',
    system: 'macOS',
    label: 'Intel',
    file: (v) => `Clipsa_${v}_x64.dmg`,
    hint: MAC_HINT,
    steps: macSteps,
    verify: macVerify
  },
  { slug: 'windows', system: 'Windows', label: 'installer (.exe)', file: (v) => `Clipsa_${v}_x64-setup.exe`, steps: windowsSteps, verify: windowsVerify },
  { slug: 'windows-msi', system: 'Windows', label: 'MSI package (.msi)', file: (v) => `Clipsa_${v}_x64_en-US.msi`, steps: windowsSteps, verify: windowsVerify },
  {
    slug: 'linux-deb',
    system: 'Linux',
    label: '.deb (Ubuntu, Debian)',
    file: (v) => `Clipsa_${v}_amd64.deb`,
    steps: (file) => linuxPackageSteps(`sudo apt install ./${file}`),
    verify: linuxVerify
  },
  {
    slug: 'linux-rpm',
    system: 'Linux',
    label: '.rpm (Fedora, openSUSE)',
    file: (v) => `Clipsa-${v}-1.x86_64.rpm`,
    steps: (file) => linuxPackageSteps(`sudo dnf install ./${file}     # Fedora\nsudo zypper install ./${file}  # openSUSE`),
    verify: linuxVerify
  },
  { slug: 'linux-appimage', system: 'Linux', label: 'AppImage (any distribution)', file: (v) => `Clipsa_${v}_amd64.AppImage`, steps: appImageSteps, verify: linuxVerify }
];

const DOWNLOAD_TEMPLATE = readFileSync(join(ROOT, 'scripts', 'download-page.html'), 'utf8');

function renderDownload(download, version) {
  const file = download.file(version);
  const release = `${REPO}/releases/tag/v${version}`;
  const sums = `${REPO}/releases/download/v${version}/SHA256SUMS`;
  const siblings = DOWNLOADS.filter((d) => d.system === download.system && d !== download);
  const others = [...siblings.map((d) => `<a href="/download/${d.slug}">${escape(d.label)}</a>`), `<a href="${release}">all files on GitHub</a>`];
  const verify = download.verify(file, sums);
  const values = {
    system: download.system,
    version,
    file,
    url: `${REPO}/releases/download/v${version}/${file}`,
    release,
    sums,
    others: `${download.hint ? `${escape(download.hint)} ` : ''}Also for ${download.system}: ${others.join(' · ')}`,
    steps: download
      .steps(file)
      .map(([title, body], i) =>
        [
          '          <li class="step">',
          `            <span class="step-num">${i + 1}</span>`,
          `            <h3>${title}</h3>`,
          `            <div class="step-body">${body}</div>`,
          '          </li>'
        ].join('\n')
      )
      .join('\n'),
    verifyCode: escape(verify.code),
    verifyNote: verify.note
  };
  return DOWNLOAD_TEMPLATE.replace(/\{\{(\w+)\}\}/g, (_, key) => {
    if (!(key in values)) throw new Error(`scripts/download-page.html: unknown placeholder {{${key}}}`);
    return values[key];
  });
}

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

if (process.argv.includes('--assets')) {
  for (const download of DOWNLOADS) console.log(download.file(latest.version));
  process.exit(0);
}

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

const downloadDir = join(SITE, 'download');
const pages = new Map(DOWNLOADS.map((d) => [`${d.slug}.html`, renderDownload(d, latest.version)]));
for (const file of existsSync(downloadDir) ? readdirSync(downloadDir).filter((f) => f.endsWith('.html')) : []) {
  if (pages.has(file)) continue;
  stale.push(`site/download/${file}`);
  if (!check) rmSync(join(downloadDir, file));
}
if (!check) mkdirSync(downloadDir, { recursive: true });
for (const [file, html] of pages) {
  const path = join(downloadDir, file);
  if (existsSync(path) && readFileSync(path, 'utf8') === html) continue;
  stale.push(`site/download/${file}`);
  if (!check) writeFileSync(path, html);
}

if (check && stale.length) {
  console.error(`Out of date with CHANGELOG.md (latest ${latest.version}): ${stale.join(', ')}. Run: pnpm site`);
  process.exit(1);
}
console.log(stale.length ? `Updated ${stale.join(', ')} (latest ${latest.version})` : `Site is up to date (latest ${latest.version})`);
