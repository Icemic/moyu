import { createHost } from '@react-spring/animated';
import { Globals, colors, createStringInterpolator, raf } from '@react-spring/shared';
import type { Node as HaiNode } from '../node';
import type { WithAnimated } from './animated';
import { primitives } from './primitives';

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
