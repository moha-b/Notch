import test from 'node:test';
import assert from 'node:assert/strict';
import '../notch/ui/sessions.js';

const snapshot = sessions => ({sessions});

test('each Claude profile reads only its own sessions', () => {
  const sessions = snapshot([
    {provider:'claude-work',state:'running'},
    {provider:'claude',state:'done'},
    {state:'attention'},
  ]);
  assert.equal(NotchSessions.workState('claude-work', sessions, []), 'running');
  assert.equal(NotchSessions.workState('claude', sessions, []), 'attention');
  assert.equal(NotchSessions.workState('claude-home', sessions, []), 'idle');
});

test('desktop network activity stands in for the default profile only', () => {
  const busy = [{provider:'claude',state:'busy'}];
  assert.equal(NotchSessions.workState('claude', snapshot([]), busy), 'running');
  assert.equal(NotchSessions.workState('claude-work', snapshot([]), busy), 'idle');
});

test('Codex profiles read their own activity rows', () => {
  const activity = [{provider:'codex-work',state:'waiting'},{provider:'gemini',state:'busy'}];
  assert.equal(NotchSessions.workState('codex-work', snapshot([]), activity), 'attention');
  assert.equal(NotchSessions.workState('codex', snapshot([]), activity), 'idle');
  assert.equal(NotchSessions.workState('antigravity', snapshot([]), activity), 'running');
});
