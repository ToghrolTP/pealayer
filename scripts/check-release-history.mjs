import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

// Checks the public release contract without downloading or executing an app.
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const history = JSON.parse(readFileSync(resolve(root, 'docs/releases/history.json'), 'utf8'));
const changelog = readFileSync(resolve(root, 'CHANGELOG.md'), 'utf8');
const template = readFileSync(resolve(root, 'docs/releases/TEMPLATE.md'), 'utf8');
const sectionHeadings = [
  '## ✨ Added and implemented',
  '## 🛠️ Fixes and improvements',
  '## 🧪 Experimental features and later follow-ups',
  '## 🐞 Bugs and problems in this build',
  '## 📦 Packages and verification',
  '## 📸 Screenshots',
  '## 🔗 Pull requests and issues',
];
for (const heading of sectionHeadings) assert(template.includes(heading), `Release template: missing ${heading}`);
const checkGit = process.argv.includes('--git');
const seenPrs = new Set();
const tags = new Set();
let previous;
let images = 0;
let packages = 0;
function ancestor(before, after) {
  if (checkGit) execFileSync('git', ['merge-base', '--is-ancestor', before, after], { cwd: root });
}
assert.equal(history.format, 'pealayer-release-history-v1');
for (const release of history.releases) {
  assert.match(release.tag, /^v\d+\.\d+\.\d+(?:-alpha\.\d+)?$/);
  assert(!tags.has(release.tag), `Duplicate release ${release.tag}`);
  tags.add(release.tag);
  assert.match(release.sha, /^[a-f0-9]{40}$/);
  assert.equal(release.prerelease, release.tag.includes('-'));
  assert.equal(release.previousSha, previous?.sha ?? null);
  assert.equal(release.previousTag, previous?.tag ?? null);
  if (previous) {
    const a = previous.tag.slice(1).split(/\D/).slice(0, 3).map(Number);
    const b = release.tag.slice(1).split(/\D/).slice(0, 3).map(Number);
    assert(b[0] > a[0] || (b[0] === a[0] && (b[1] > a[1] || (b[1] === a[1] && b[2] > a[2]))), 'Versions must increase');
    ancestor(previous.sha, release.sha);
  }
  ancestor(release.sha, history.currentPinnedSource);
  const notes = readFileSync(resolve(root, release.notes), 'utf8');
  assert.match(release.emoji, /\p{Extended_Pictographic}/u, `${release.tag}: missing milestone emoji`);
  assert.equal(notes.split(/\r?\n/, 1)[0], `# ${release.emoji} Pealayer ${release.tag} — ${release.title}`);
  assert(notes.includes(`/commit/${release.sha}`));
  for (const required of ['[Added]', '[Fixed]', '[Experimental]', '[Known issue]', ...sectionHeadings]) {
    assert(notes.includes(required), `${release.tag}: missing ${required}`);
  }
  assert(notes.includes(`<!-- pealayer-release-backfill:${release.tag} -->`));
  assert(changelog.includes(`## ${release.emoji} ${release.tag} `));
  assert(release.screenshots.length > 0);
  for (const shot of release.screenshots) {
    assert(!shot.path.includes('..'));
    const file = resolve(root, shot.path);
    const bytes = readFileSync(file);
    assert.equal(createHash('sha256').update(bytes).digest('hex'), shot.sha256, `Screenshot changed: ${shot.path}`);
    assert(notes.includes(shot.immutableUrl), `Missing rendered image ${shot.path}`);
    const imageCommit = shot.imageCommit ?? history.imageCommit;
    assert.match(imageCommit, /^[a-f0-9]{40}$/);
    assert.equal(shot.immutableUrl, `https://raw.githubusercontent.com/ToghrolTP/pealayer/${imageCommit}/${shot.path}`);
    assert(existsSync(resolve(root, shot.evidence)), `Missing image provenance ${shot.evidence}`);
    ancestor(shot.sourceSha, release.sha);
    images++;
  }
  for (const pr of release.prs) {
    assert(!seenPrs.has(pr.number), `PR #${pr.number} assigned twice`);
    seenPrs.add(pr.number);
    assert(notes.includes(pr.url), `PR #${pr.number} omitted from ${release.tag}`);
    ancestor(pr.mergeCommit, release.sha);
  }
  for (const pkg of release.packages) {
    assert.match(pkg.sha256, /^[a-f0-9]{64}$/);
    assert.equal(pkg.sourceSha, release.sha);
    assert(pkg.file.includes(release.tag) && pkg.file.includes(release.sha.slice(0, 7)));
    assert(notes.includes(`/releases/download/${release.tag}/${pkg.file}`));
    packages++;
  }
  previous = release;
}
assert.equal(previous.sha, history.currentPinnedSource);
console.log(`Release history verified: ${tags.size} versions, ${seenPrs.size} merged PRs, ${images} images, ${packages} native packages${checkGit ? ', Git ancestry checked' : ''}.`);
