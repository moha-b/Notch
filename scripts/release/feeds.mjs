import fs from 'node:fs';
import crypto from 'node:crypto';
import {validateAssets} from './policy.mjs';

export function feeds(manifest, assets, notes) {
  validateAssets(manifest, assets);
  const xml = text => String(text).replace(/[<>&"']/g, character => ({'<':'&lt;','>':'&gt;','&':'&amp;','"':'&quot;',"'":'&apos;'}[character]));
  const windows = assets.find(asset => asset.name.endsWith('.exe'));
  const macos = assets.find(asset => asset.name.endsWith('.dmg'));
  const prefix = `https://github.com/moha-b/Notch/releases/download/v${manifest.version}/`;
  return {
    windows: {version:manifest.version,notes,platforms:{'windows-x86_64':{url:prefix+windows.name,signature:windows.signature}}},
    macos: `<?xml version="1.0" encoding="utf-8"?>\n<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle"><channel><title>Notch</title><item><title>Notch ${xml(manifest.version)}</title><description>${xml(notes)}</description><sparkle:version>${manifest.macBuild}</sparkle:version><sparkle:shortVersionString>${xml(manifest.version)}</sparkle:shortVersionString><sparkle:minimumSystemVersion>15.0</sparkle:minimumSystemVersion><enclosure url="${prefix+macos.name}" length="${macos.size}" type="application/octet-stream" sparkle:edSignature="${xml(macos.signature)}"/></item></channel></rss>\n`,
  };
}

export function assetRecord(directory, name) {
  const bytes = fs.readFileSync(`${directory}/${name}`);
  let signature = fs.readFileSync(`${directory}/${name}.sig`,'utf8').trim();
  if (name.endsWith('.dmg')) signature = signature.match(/sparkle:edSignature="([^"]+)"/)?.[1] || '';
  return {name,size:bytes.length,sha256:crypto.createHash('sha256').update(bytes).digest('hex'),signature};
}
