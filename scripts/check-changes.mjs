#!/usr/bin/env node
// Checks the release notes waiting in .changes/ (see .changes/README.md) and
// lists them. CI runs it on every pull request, so a malformed note fails the
// PR instead of the release PR that covector builds from it.
//
//   node scripts/check-changes.mjs
//
// In GitHub Actions the list also goes to the job summary.

import { appendFileSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIR = join(dirname(fileURLToPath(import.meta.url)), '..', '.changes');
const config = JSON.parse(readFileSync(join(DIR, 'config.json'), 'utf8'));
const packages = Object.keys(config.packages);
const tags = Object.keys(config.changeTags);
const BUMPS = ['patch', 'minor', 'major'];

const errors = [];
const notes = [];
for (const file of readdirSync(DIR).filter((f) => f.endsWith('.md') && f !== 'README.md').sort()) {
  const text = readFileSync(join(DIR, file), 'utf8');
  const match = text.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
  if (!match) {
    errors.push(`${file}: must start with a --- block (see .changes/README.md)`);
    continue;
  }
  const [, header, body] = match;
  const entries = header.split('\n').filter((line) => line.trim());
  const before = errors.length;
  if (entries.length === 0) errors.push(`${file}: the --- block names no package`);
  if (!body.trim()) errors.push(`${file}: the note is empty; describe what changes for users`);
  const parsed = [];
  for (const line of entries) {
    const entry = line.match(/^\s*["']?([^"':]+)["']?\s*:\s*["']?(\w+)(?::(\w+))?["']?\s*$/);
    if (!entry) {
      errors.push(`${file}: can't read "${line}"; expected "clipsa": patch:fixed`);
      continue;
    }
    const [, name, bump, tag] = entry;
    if (!packages.includes(name)) errors.push(`${file}: unknown package "${name}" (expected ${packages.join(', ')})`);
    if (!BUMPS.includes(bump)) errors.push(`${file}: "${bump}" isn't one of ${BUMPS.join(', ')}`);
    if (!tag) errors.push(`${file}: add a section after the bump, e.g. ${bump}:fixed (one of ${tags.join(', ')})`);
    else if (!tags.includes(tag)) errors.push(`${file}: unknown section "${tag}" (one of ${tags.join(', ')})`);
    parsed.push({ file, bump, section: config.changeTags[tag], text: body.trim() });
  }
  if (errors.length === before) notes.push(...parsed);
}

const order = ['major', 'minor', 'patch'];
const bump = notes.map((n) => n.bump).sort((a, b) => order.indexOf(a) - order.indexOf(b))[0];
const summary = notes.length
  ? [
      `### Next release: ${bump} (${notes.length} note${notes.length === 1 ? '' : 's'} in .changes/)`,
      '',
      ...notes.map((n) => `- **${n.section}** (\`${n.file}\`): ${n.text.replace(/\s*\n\s*/g, ' ')}`)
    ].join('\n')
  : '### No release notes waiting in .changes/';
console.log(summary);
if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${summary}\n`);

if (errors.length) {
  for (const error of errors) console.error(`::error file=.changes/${error.split(':')[0]}::${error}`);
  process.exit(1);
}
