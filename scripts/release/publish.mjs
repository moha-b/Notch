import fs from 'node:fs';
import crypto from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {assetRecord, feeds} from './feeds.mjs';

const directory = 'work/release';
const manifest = JSON.parse(fs.readFileSync(`${directory}/release-manifest.json`,'utf8'));
const tag = `v${manifest.version}`;
const gh = (...args) => execFileSync('gh', [...args,'--repo','moha-b/Notch'],{encoding:'utf8'}).trim();
const notes = execFileSync('gh',['api','repos/moha-b/Notch/releases/generate-notes','-f',`tag_name=${tag}`,'-f',`target_commitish=${manifest.source}`],{encoding:'utf8'});
fs.writeFileSync(`${directory}/release-notes.md`,JSON.parse(notes).body);
const assets = ['windows-x64-setup.exe','macos-arm64.dmg'].map(suffix => assetRecord(directory,`Notch-${manifest.version}-${suffix}`));
const generated = feeds(manifest,assets,JSON.parse(notes).body);
fs.writeFileSync(`${directory}/SHA256SUMS.txt`,assets.map(asset => `${asset.sha256}  ${asset.name}`).join('\n')+'\n');
const filenames = assets.flatMap(asset => [asset.name,`${asset.name}.sig`]).concat(['SHA256SUMS.txt','release-manifest.json','release-notes.md']);
// Creating, rather than updating, makes duplicate and concurrent publication fail closed.
gh('release','create',tag,'--target',manifest.source,'--draft','--title',`Notch ${manifest.version}`,'--notes-file',`${directory}/release-notes.md`,...filenames.map(name=>`${directory}/${name}`));
const draft = JSON.parse(gh('release','view',tag,'--json','assets,isDraft'));
if (!draft.isDraft || filenames.some(name => !draft.assets.some(asset => asset.name === name))) throw new Error('Draft release is missing required assets.');
gh('release','edit',tag,'--draft=false','--latest');
const published = JSON.parse(gh('release','view',tag,'--json','assets,isDraft'));
if (published.isDraft) throw new Error('Release is not public; feeds stay unchanged.');
for (const asset of assets) {
  const publicAsset = published.assets.find(candidate => candidate.name === asset.name);
  if (!publicAsset || publicAsset.size !== asset.size) throw new Error('Published asset does not match the signed package.');
  const response = await fetch('https://github.com/moha-b/Notch/releases/download/'+tag+'/'+asset.name);
  if (!response.ok) throw new Error('Published download is not reachable; previous feeds remain active.');
  const digest = crypto.createHash('sha256').update(Buffer.from(await response.arrayBuffer())).digest('hex');
  if (digest !== asset.sha256) throw new Error('Public download checksum mismatch; previous feeds remain active.');
}
fs.mkdirSync('work/pages/updates',{recursive:true});
fs.writeFileSync('work/pages/updates/windows.json',JSON.stringify(generated.windows,null,2)+'\n');
fs.writeFileSync('work/pages/updates/macos.xml',generated.macos);
fs.writeFileSync('work/pages/index.html',`<!doctype html><meta charset="utf-8"><title>Notch</title><h1>Notch</h1><p>Native Mac and Windows usage monitor.</p><p><a href="https://github.com/moha-b/Notch/releases/tag/${tag}">Download Notch ${manifest.version}</a></p>`);
