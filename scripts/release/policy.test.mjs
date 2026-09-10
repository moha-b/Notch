import test from 'node:test';
import assert from 'node:assert/strict';
import {releaseManifest, versionParts, qualify, validateAssets, verifyPublication} from './policy.mjs';

const source = 'a'.repeat(40);
test('release numbers are stable, monotonic, and start at 1.0.0', () => {
  const first = releaseManifest({version:'1.0.0',source});
  assert.equal(first.windowsVersion, '1.0.0.0');
  assert.equal(releaseManifest({version:'1.0.1',source}, first).macBuild, 2);
  for (const version of ['1.0.0','0.9.9']) assert.throws(() => releaseManifest({version,source}, first));
  assert.throws(() => releaseManifest({version:'1.0.1',source}));
  for (const version of ['v1.0.0','1.0.0-beta','01.0.0','1.0','65536.0.0']) assert.throws(() => versionParts(version));
});
test('missing platform qualification or a different source prevents publication', () => {
  const report = {source};
  for (const gate of ['providers','profiles','credentials','layout','sessions','settings','install','upgrade','uninstall','updates','screenshots']) {
    report[gate] = {macos:'passed',windows:'passed'};
  }
  qualify(report, source);
  assert.throws(() => qualify(report, 'b'.repeat(40)));
  report.updates.windows = 'pending';
  assert.throws(() => qualify(report, source), /updates/);
});
test('a missing platform or updater signature prevents feed generation', () => {
  const manifest = releaseManifest({version:'1.0.0',source});
  const assets = ['windows-x64-setup.exe','macos-arm64.dmg'].map(suffix => ({name:`Notch-1.0.0-${suffix}`,size:100,sha256:'b'.repeat(64),signature:'fixture-signature'}));
  validateAssets(manifest, assets);
  assert.throws(() => validateAssets(manifest, assets.slice(0,1)));
  assets[0].signature = '';
  assert.throws(() => validateAssets(manifest, assets));
});
test('feed retry accepts only the same public build', () => {
  const manifest = releaseManifest({version:'1.0.0',source});
  const publication = {draft:false,prerelease:false};
  verifyPublication(manifest, structuredClone(manifest), publication);
  for (const changed of [{source:'b'.repeat(40)},{macBuild:2},{version:'1.0.1'},{targets:['windows-x64']}]) {
    assert.throws(() => verifyPublication(manifest, {...manifest,...changed}, publication));
  }
  for (const changed of [{draft:true},{prerelease:true}]) {
    assert.throws(() => verifyPublication(manifest, manifest, {...publication,...changed}));
  }
});
