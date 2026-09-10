export function versionParts(version) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) throw new Error('Use a stable X.Y.Z version.');
  const parts = version.split('.').map(Number);
  if (parts.some(part => part > 65535)) throw new Error('Windows version components must fit unsigned 16-bit integers.');
  return parts;
}

export function compareVersions(left, right) {
  const previous = versionParts(right);
  const next = versionParts(left);
  for (let index = 0; index < 3; index++) {
    if (next[index] !== previous[index]) return Math.sign(next[index] - previous[index]);
  }
  return 0;
}

export function releaseManifest(request, previous) {
  versionParts(request.version);
  if (!/^[a-f0-9]{40}$/.test(request.source)) throw new Error('Select a full 40-character source commit SHA.');
  if (!previous && request.version !== '1.0.0') throw new Error('The first Notch release must be 1.0.0.');
  if (previous && compareVersions(request.version, previous.version) <= 0) throw new Error('Published versions cannot be reused or decreased.');
  if (previous && (!Number.isSafeInteger(previous.macBuild) || previous.macBuild < 1)) throw new Error('Previous Mac build number is invalid.');
  return {product:'Notch', version:request.version, source:request.source,
    macBuild:previous ? previous.macBuild + 1 : 1, windowsVersion:`${request.version}.0`,
    targets:['macos-arm64','windows-x64']};
}

export function qualify(report, source) {
  const required = ['providers','profiles','credentials','layout','sessions','settings','install','upgrade','uninstall','updates','screenshots'];
  if (report.source !== source) throw new Error('Qualification evidence must match the selected source SHA.');
  for (const gate of required) {
    if (report[gate]?.macos !== 'passed' || report[gate]?.windows !== 'passed') throw new Error(`Release blocked: ${gate} is not qualified on both platforms.`);
  }
}

export function validateAssets(manifest, assets) {
  const expected = [`Notch-${manifest.version}-windows-x64-setup.exe`, `Notch-${manifest.version}-macos-arm64.dmg`];
  for (const name of expected) {
    const asset = assets.find(asset => asset.name === name);
    if (!asset || asset.size <= 0 || !/^[a-f0-9]{64}$/.test(asset.sha256) || !asset.signature?.trim()) {
      throw new Error(`Missing or invalid signed asset: ${name}`);
    }
  }
  if (assets.length !== expected.length) throw new Error('Unexpected release platform assets.');
}
