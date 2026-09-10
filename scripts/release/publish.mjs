import fs from 'node:fs';
import crypto from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {assetRecord, feeds} from './feeds.mjs';
import {compareVersions, verifyPublication, versionParts} from './policy.mjs';

const directory = 'work/release';
const repository = 'moha-b/Notch';
const manifest = JSON.parse(fs.readFileSync(`${directory}/release-manifest.json`,'utf8'));
versionParts(manifest.version);
const tag = `v${manifest.version}`;
const gh = (...args) => execFileSync('gh', args, {encoding:'utf8'}).trim();
const api = (...args) => JSON.parse(gh('api',...args));
const release = (...args) => gh('release',...args,'--repo',repository);
const assets = ['windows-x64-setup.exe','macos-arm64.dmg'].map(suffix => assetRecord(directory,`Notch-${manifest.version}-${suffix}`));
const filenames = assets.flatMap(asset => [asset.name,`${asset.name}.sig`]).concat(['SHA256SUMS.txt','release-manifest.json','release-notes.md']);
const releases = api('--paginate','--slurp',`repos/${repository}/releases?per_page=100`).flat();
const existing = releases.find(published => published.tag_name === tag);

function publishPackages() {
  const notes = api(`repos/${repository}/releases/generate-notes`,'-f',`tag_name=${tag}`,'-f',`target_commitish=${manifest.source}`).body;
  feeds(manifest,assets,notes);
  fs.writeFileSync(`${directory}/release-notes.md`,notes);
  fs.writeFileSync(`${directory}/SHA256SUMS.txt`,assets.map(asset => `${asset.sha256}  ${asset.name}`).join('\n')+'\n');
  // GitHub rejects an existing ref atomically, even when another publisher wins the race.
  api(`repos/${repository}/git/refs`,'-f',`ref=refs/tags/${tag}`,'-f',`sha=${manifest.source}`);
  release('create',tag,'--verify-tag','--draft','--title',`Notch ${manifest.version}`,'--notes-file',`${directory}/release-notes.md`,...filenames.map(name=>`${directory}/${name}`));
  const draft = api(`repos/${repository}/releases/tags/${tag}`);
  if (!draft.draft || filenames.some(name => !draft.assets.some(asset => asset.name === name))) throw new Error('Draft release is missing required assets.');
  release('edit',tag,'--draft=false','--latest');
  return api(`repos/${repository}/releases/tags/${tag}`);
}

async function publicBytes(name) {
  const response = await fetch(`https://github.com/${repository}/releases/download/${tag}/${name}`);
  if (!response.ok) throw new Error(`Published ${name} is not reachable; previous feeds remain active.`);
  return Buffer.from(await response.arrayBuffer());
}

async function verifyPackages(publication) {
  for (const asset of assets) {
    const remote = publication.assets.find(candidate => candidate.name === asset.name);
    if (!remote || remote.size !== asset.size) throw new Error('Published asset does not match the signed package.');
    const digest = crypto.createHash('sha256').update(await publicBytes(asset.name)).digest('hex');
    if (digest !== asset.sha256) throw new Error('Public download checksum mismatch; previous feeds remain active.');
    const signature = await publicBytes(`${asset.name}.sig`);
    if (!signature.equals(fs.readFileSync(`${directory}/${asset.name}.sig`))) throw new Error('Published signature differs from this build.');
  }
}

async function verifiedNotes(publication) {
  const publishedManifest = JSON.parse((await publicBytes('release-manifest.json')).toString('utf8'));
  verifyPublication(manifest,publishedManifest,publication);
  const reference = api(`repos/${repository}/git/ref/tags/${tag}`);
  if (reference.object.type !== 'commit' || reference.object.sha !== manifest.source) throw new Error('Published tag does not identify the selected source.');
  await verifyPackages(publication);
  const checksums = assets.map(asset => `${asset.sha256}  ${asset.name}`).join('\n')+'\n';
  if ((await publicBytes('SHA256SUMS.txt')).toString('utf8') !== checksums) throw new Error('Published checksums differ from this build.');
  const notes = (await publicBytes('release-notes.md')).toString('utf8');
  if (notes !== publication.body) throw new Error('Published release notes differ from the immutable notes asset.');
  return notes;
}

function writePages(notes) {
  const generated = feeds(manifest,assets,notes);
  fs.mkdirSync('work/pages/updates',{recursive:true});
  fs.writeFileSync('work/pages/updates/windows.json',JSON.stringify(generated.windows,null,2)+'\n');
  fs.writeFileSync('work/pages/updates/macos.xml',generated.macos);
  fs.writeFileSync('work/pages/index.html',`<!doctype html><meta charset="utf-8"><title>Notch</title><h1>Notch</h1><p>Native Mac and Windows usage monitor.</p><p><a href="https://github.com/${repository}/releases/tag/${tag}">Download Notch ${manifest.version}</a></p>`);
}

const newer = releases.some(published => !published.draft && !published.prerelease && published.assets.some(asset => asset.name === 'release-manifest.json') && compareVersions(published.tag_name.slice(1),manifest.version) > 0);
if (newer) throw new Error('A newer version is already public; feeds cannot move backwards.');
if (existing?.draft) throw new Error('An existing draft reserves this version. Inspect it before taking further action.');
const publication = existing || publishPackages();
writePages(await verifiedNotes(publication));
