import test from 'node:test';
import assert from 'node:assert/strict';
import '../notch/ui/visibility.js';

test('only Always show keeps the card open', () => {
  assert.equal(NotchVisibility.keepsCardOpen('always_show'), true);
  for (const mode of ['on_hover', 'hidden']) assert.equal(NotchVisibility.keepsCardOpen(mode), false, mode);
});

test('Hide suppresses peeks while the visible modes allow them', () => {
  assert.equal(NotchVisibility.allowsPeek('hidden'), false);
  for (const mode of ['always_show', 'on_hover']) assert.equal(NotchVisibility.allowsPeek(mode), true, mode);
});

test('only the main display window plays sounds', () => {
  assert.equal(NotchVisibility.playsSounds('notch'), true);
  assert.equal(NotchVisibility.playsSounds('notch-2'), false);
});
