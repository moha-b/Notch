import test from 'node:test';
import assert from 'node:assert/strict';
import '../notch/ui/geometry.js';

test('one or twelve providers stay inside every edge at extreme offsets', () => {
  for (const edge of ['left','right','top','bottom']) {
    const horizontal = ['top','bottom'].includes(edge);
    const viewport = horizontal ? {width:1280,height:460} : {width:340,height:1080};
    for (const count of [1,12]) for (const ratio of [0,0.5,1]) {
      const size = horizontal ? {width:count===1?94:923,height:101} : {width:70,height:count===1?117:1000};
      const strip = {...NotchGeometry.pill({edge,ratio,viewport,size}),...size};
      assert.ok(strip.left>=0 && strip.top>=0);
      assert.ok(strip.left+strip.width<=viewport.width && strip.top+strip.height<=viewport.height);
      const cardSize={width:225,height:300};
      const card=NotchGeometry.card({edge,viewport,pill:strip,cell:strip,size:cardSize});
      assert.ok(card.left>=0 && card.top>=0);
      assert.ok(card.left+cardSize.width<=viewport.width && card.top+cardSize.height<=viewport.height);
      if(horizontal) assert.ok(edge==='top'?card.top>=strip.top+strip.height:card.top+cardSize.height<=strip.top);
      else assert.ok(edge==='left'?card.left>=strip.left+strip.width:card.left+cardSize.width<=strip.left);
    }
  }
});
