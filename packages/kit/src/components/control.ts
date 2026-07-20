import type { TextProps } from '../bindings/TextProps';
import type { MoyuSpriteAttributes } from '../declaration';

export type ControlState = 'idle' | 'hover' | 'press' | 'disabled';

export type ControlStateValue<T> = T | readonly [idle: T, hover?: T, press?: T, disabled?: T];

export type ControlTextStyle = Omit<TextProps, 'text'>;

export interface ControlSpriteProps extends Omit<MoyuSpriteAttributes, 'children' | 'src'> {
  src: ControlStateValue<string>;
}

export function resolveControlStateValue<T>(value: ControlStateValue<T>, state: ControlState): T {
  if (!Array.isArray(value)) {
    return value as T;
  }

  const index = state === 'idle' ? 0 : state === 'hover' ? 1 : state === 'press' ? 2 : 3;
  return value[index] ?? value[0];
}
