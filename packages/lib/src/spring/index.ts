import { Globals, createStringInterpolator, colors, raf } from '@react-spring/shared';
import { createHost } from '@react-spring/animated';
import { primitives } from './primitives';
import { WithAnimated } from './animated';
import type { Node as HaiNode } from '../node';

Globals.assign({
  createStringInterpolator,
  colors,
  frameLoop: 'demand',
});

// Let r3f drive the frameloop.
setInterval(() => {
  raf.advance();
}, 10);

const host = createHost(primitives, {
  applyAnimatedValues(instance: HaiNode, props) {
    instance.updateProps(props);
  },
});

export const animated = host.animated as WithAnimated;
export { animated as a };

export * from './animated';
export * from '@react-spring/core';
