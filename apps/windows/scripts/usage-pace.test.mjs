import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import '../notch/ui/usage-pace.js';

const now = 1_800_000_000_000;
const fixtures = JSON.parse(fs.readFileSync(new URL('../../../shared/usage-pace-fixtures.json', import.meta.url)));

test('shared native pace contract handles cycles, resets and unavailable readings', () => {
  for (const fixture of fixtures) {
    const pace = NotchUsagePace({used:fixture.used, duration_seconds:fixture.duration,
      resets_at:fixture.remaining == null ? null : now + fixture.remaining * 1000}, now);
    if (fixture.points == null) assert.equal(pace, null, fixture.name);
    else {
      assert.ok(Math.abs(pace.points - fixture.points) < 0.000001, fixture.name);
      assert.equal(pace.summary, fixture.summary, fixture.name);
      assert.equal(pace.deficit, fixture.points > 0, fixture.name);
    }
  }
});

test('non-finite input and count-only windows never produce a pace', () => {
  const window = {used:0.5,duration_seconds:604800,resets_at:now + 302400000};
  for (const invalid of [{used:NaN},{used:Infinity},{duration_seconds:Infinity},
    {resets_at:Infinity},{count:100}]) assert.equal(NotchUsagePace({...window,...invalid},now), null);
});
