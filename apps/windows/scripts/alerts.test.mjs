import test from 'node:test';
import assert from 'node:assert/strict';
import '../notch/ui/alerts.js';

const preferences = {muted_providers:[],announce_completion:true,sounds:false};
const provider = used => ({id:'claude',name:'Claude',snap:{status:'ok',windows:[{used}]}});

test('threshold alerts fire on crossing, rearm after reset, and ignore stale counts', () => {
  const alerts = new NotchAlerts();
  const observe = reading => alerts.observe([reading], preferences, {claude:'idle'});
  assert.equal(observe(provider(0.79)).length, 0);
  assert.equal(observe(provider(0.8)).length, 1);
  assert.equal(observe(provider(0.91)).length, 0);
  assert.equal(observe(provider(1)).length, 1);
  assert.equal(observe(provider(1)).length, 0);
  assert.equal(observe({...provider(0),snap:{status:'stale',windows:[{used:0}]}}).length, 0);
  assert.equal(observe(provider(1)).length, 0);
  observe(provider(0.1));
  assert.equal(observe(provider(1)).length, 2);
  assert.equal(observe({...provider(0),snap:{status:'ok',windows:[{used:1,count:100}]}}).length, 0);
});

test('muting consumes crossings without replaying them on unmute', () => {
  const alerts = new NotchAlerts();
  assert.deepEqual(alerts.observe([provider(0.9)], {...preferences,muted_providers:['claude']}, {claude:'idle'}), []);
  assert.deepEqual(alerts.observe([provider(0.9)], preferences, {claude:'idle'}), []);
  assert.equal(alerts.observe([provider(1)], preferences, {claude:'idle'}).length, 1);
});

test('session peeks need a real transition and obey preferences', () => {
  for (const state of ['idle','done','attention']) {
    const alerts = new NotchAlerts();
    const observe = state => alerts.observe([provider(0)], preferences, {claude:state});
    assert.deepEqual(observe('running'), []);
    assert.equal(observe(state)[0].kind, 'session');
    assert.deepEqual(observe(state), []);
  }
  const alerts = new NotchAlerts();
  alerts.observe([provider(0)], preferences, {claude:'running'});
  assert.deepEqual(alerts.observe([provider(0)], {...preferences,announce_completion:false,sounds:false}, {claude:'done'}), []);
  alerts.observe([], preferences, {});
  assert.deepEqual(alerts.observe([provider(0)], preferences, {claude:'attention'}), []);
});
