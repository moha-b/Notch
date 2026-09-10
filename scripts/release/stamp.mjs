import fs from 'node:fs';
import {releaseManifest, versionParts} from './policy.mjs';

const manifest = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
// Reuse input validation without applying first-release sequencing a second time.
releaseManifest({version:'1.0.0',source:manifest.source});
versionParts(manifest.version);
if (!Number.isSafeInteger(manifest.macBuild) || manifest.macBuild < 1) throw new Error('Invalid Mac build number.');
const publicKeys = {tauri:process.env.NOTCH_TAURI_PUBLIC_KEY, sparkle:process.env.NOTCH_SPARKLE_PUBLIC_KEY};
if (!publicKeys.tauri || !publicKeys.sparkle) throw new Error('Both product-owned updater public keys are required.');
const edit = (path, transform) => fs.writeFileSync(path, transform(fs.readFileSync(path,'utf8')));
for (const name of ['notch','notch-hook']) {
  edit(`apps/windows/${name}/Cargo.toml`, text => text.replace(/^version = "[^"]+"/m, `version = "${manifest.version}"`));
  edit('apps/windows/Cargo.lock', text => text.replace(new RegExp(`(name = "${name}"\\r?\\nversion = ")[^"]+`), (_, prefix) => `${prefix}${manifest.version}`));
}
edit('apps/windows/notch/tauri.conf.json', text => {
  const config = JSON.parse(text);
  Object.assign(config,{productName:'Notch',version:manifest.version,identifier:'io.github.moha-b.notch'});
  config.bundle.createUpdaterArtifacts = true;
  config.plugins = {updater:{pubkey:publicKeys.tauri,endpoints:['https://moha-b.github.io/Notch/updates/windows.json'],windows:{installMode:'passive'}}};
  return JSON.stringify(config,null,2)+'\n';
});
edit('apps/macos/project.yml', text => text.replace(/MARKETING_VERSION: "[^"]+"/, `MARKETING_VERSION: "${manifest.version}"`)
  .replace(/CURRENT_PROJECT_VERSION: "[^"]+"/, `CURRENT_PROJECT_VERSION: "${manifest.macBuild}"`)
  .replaceAll('io.github.moha-b.notch.preview','io.github.moha-b.notch').replace('CFBundleDisplayName: Notch Preview','CFBundleDisplayName: Notch')
  .replace('NotchChannel: development','NotchChannel: stable').replace('SUEnableAutomaticChecks: false','SUEnableAutomaticChecks: true')
  .replace('SUPublicEDKey: ""',`SUPublicEDKey: "${publicKeys.sparkle}"`));
