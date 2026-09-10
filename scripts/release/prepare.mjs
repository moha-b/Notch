import fs from 'node:fs';
import {execFileSync} from 'node:child_process';
import {releaseManifest, qualify, compareVersions} from './policy.mjs';

const repository = 'moha-b/Notch';
const gh = (...args) => execFileSync('gh', args, {encoding:'utf8'}).trim();
const api = endpoint => JSON.parse(gh('api', `repos/${repository}/${endpoint}`));
const [source, version, qualificationPath] = process.argv.slice(2);
if (!/^[a-f0-9]{40}$/.test(source || '')) throw new Error('Select a full source SHA.');
execFileSync('git', ['merge-base','--is-ancestor',source,'origin/main']);
qualify(JSON.parse(fs.readFileSync(qualificationPath, 'utf8')), source);
const notes = JSON.parse(execFileSync('git', ['show', `${source}:shared/release-notes.json`], {encoding:'utf8'}));
if (!notes.some(note => note.version === version)) throw new Error('The selected source has no embedded release notes for this version.');

const releases = JSON.parse(gh('api','--paginate','--slurp',`repos/${repository}/releases?per_page=100`)).flat();
if (releases.some(release => release.tag_name === `v${version}`)) throw new Error('Version already exists, including drafts. Never overwrite its assets.');
const previousReleases = releases.filter(release => !release.draft && !release.prerelease && release.assets.some(asset => asset.name === 'release-manifest.json'));
previousReleases.sort((left,right) => compareVersions(right.tag_name.slice(1),left.tag_name.slice(1)));
let previous;
if (previousReleases.length) {
  const asset = previousReleases[0].assets.find(asset => asset.name === 'release-manifest.json');
  previous = JSON.parse(gh('api',`repos/${repository}/releases/assets/${asset.id}`,'-H','Accept: application/octet-stream'));
  execFileSync('git', ['merge-base','--is-ancestor',previous.source,source]);
}

const runs = api(`actions/runs?head_sha=${source}&status=success&event=push&per_page=100`).workflow_runs;
const qualified = runs.some(run => {
  if (run.name !== 'Native candidates' || run.head_branch !== 'main') return false;
  const jobs = api(`actions/runs/${run.id}/jobs?per_page=100`).jobs;
  return ['windows','macos'].every(name => jobs.some(job => job.name === name && job.conclusion === 'success'));
});
if (!qualified) throw new Error('Both native candidate lanes must pass on the selected main commit.');
const manifest = releaseManifest({version,source}, previous);
fs.mkdirSync('work/release', {recursive:true});
fs.writeFileSync('work/release/release-manifest.json', JSON.stringify(manifest,null,2)+'\n');
console.log(`Prepared Notch ${manifest.version} from ${manifest.source}; Mac build ${manifest.macBuild}.`);
