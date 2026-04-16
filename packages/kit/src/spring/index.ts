import { createHost } from '@react-spring/animated';
import { Globals, colors, createStringInterpolator } from '@react-spring/shared';
import type { Node as MoyuNode } from '../node';
import type { WithAnimated } from './animated';
import { primitives } from './primitives';

Globals.assign({
  createStringInterpolator,
  colors,
  frameLoop: 'always',
  requestAnimationFrame: (cb) => requestAnimationFrame(cb),
});

const host = createHost(primitives, {
  applyAnimatedValues(instance: MoyuNode, props) {
    instance.updateProps(props);
  },
});

export const animated = host.animated as WithAnimated;
export { animated as a };

export * from './animated';
export * from '@react-spring/core';
