globalThis.NotchGeometry = {
  clamp(start, length, available, margin) {
    const inset = Math.min(margin, Math.max(0, (available - length) / 2));
    return Math.min(Math.max(inset, start), Math.max(inset, available - length - inset));
  },

  pill({edge, ratio, viewport, size}) {
    const horizontal = edge === 'top' || edge === 'bottom';
    const length = horizontal ? size.width : size.height;
    const available = horizontal ? viewport.width : viewport.height;
    const position = this.clamp(available * ratio - length / 2, length, available, 40);
    return horizontal
      ? {left:position,top:edge === 'top' ? 0 : viewport.height-size.height}
      : {left:edge === 'left' ? 0 : viewport.width-size.width,top:position};
  },

  card({edge, viewport, pill, cell, size}) {
    const horizontal = edge === 'top' || edge === 'bottom';
    const left = horizontal ? cell.left + cell.width/2 - size.width/2
      : edge === 'left' ? pill.left+pill.width+12 : pill.left-size.width-12;
    const top = horizontal ? (edge === 'top' ? pill.top+pill.height+12 : pill.top-size.height-12)
      : cell.top+cell.height/2-size.height/2;
    return {left:this.clamp(left,size.width,viewport.width,8),top:this.clamp(top,size.height,viewport.height,8)};
  },
};
