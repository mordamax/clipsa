#!/usr/bin/env node
// Runs after `covector version` has bumped src-tauri/Cargo.toml and added the
// new release section to CHANGELOG.md (covector's `postversion`, see
// .changes/config.json). It turns that section into the format the release
// workflow and the website read, then updates the files that depend on it:
//
// - `## [X.Y.Z]` → `## [X.Y.Z] - YYYY-MM-DD` (today, UTC): only dated
//   sections count as released (scripts/build-site.mjs, release.yml).
// - Each entry ``- [`abc1234`](commit) ([#12](pr) by [@who](user)) Note`` →
//   `- Note ([#12](pr))`: the note is written for users; the PR link stays for
//   anyone who wants the details.
// - Blank lines inside a note are dropped, so a note stays one bullet.
// - `cargo update -p clipsa` (Cargo.lock) and the generated site pages.
//
//   node scripts/finish-release.mjs

import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const CHANGELOG = join(ROOT, 'CHANGELOG.md');

const cargo = readFileSync(join(ROOT, 'src-tauri/Cargo.toml'), 'utf8');
const version = cargo.match(/^\[package\][^[]*?^version = "([^"]+)"/m)?.[1];
if (!version) throw new Error('src-tauri/Cargo.toml has no [package] version');

const today = new Date().toISOString().slice(0, 10);
const lines = readFileSync(CHANGELOG, 'utf8').split('\n');
const start = lines.findIndex((line) => line === `## [${version}]`);
if (start === -1) throw new Error(`CHANGELOG.md has no undated "## [${version}]" section to finish`);
let end = lines.findIndex((line, i) => i > start && line.startsWith('## ['));
if (end === -1) end = lines.length;

const ENTRY = /^- (?:\[`[0-9a-f]+`\]\([^)]*\) ?)?(?:\((\[#\d+\]\([^)]*\))[^)]*\)\)? ?)?(.*)$/;
const section = [`## [${version}] - ${today}`];
// The PR link of the note being copied; it goes after the note's last line.
let link = null;
const endNote = () => {
  if (link) section[section.length - 1] += ` (${link})`;
  link = null;
};
for (let i = start + 1; i < end; i++) {
  const line = lines[i];
  const entry = line.startsWith('- ') && line.match(ENTRY);
  if (entry) {
    endNote();
    section.push(`- ${entry[2]}`);
    link = entry[1] ?? null;
  } else if (/^\s+\S/.test(line) && /^(- |\s+\S)/.test(section.at(-1))) {
    section.push(line); // the note continues
  } else if (!line.trim() && /^\s+\S/.test(lines[i + 1] ?? '') && /^(- |\s+\S)/.test(section.at(-1))) {
    // A blank line inside the note: dropped, so the note stays one bullet.
  } else {
    endNote();
    section.push(line);
  }
}
endNote();

lines.splice(start, end - start, ...section);
writeFileSync(CHANGELOG, lines.join('\n'));
console.log(`CHANGELOG.md: finished ${version} (${today})`);

const run = (cmd, args) => execFileSync(cmd, args, { cwd: ROOT, stdio: 'inherit' });
run('cargo', ['update', '-p', 'clipsa', '--manifest-path', 'src-tauri/Cargo.toml']);
run(process.execPath, ['scripts/build-site.mjs']);
