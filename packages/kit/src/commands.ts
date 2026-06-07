import type { AudioCommand } from './bindings/AudioCommand';
import type { GamepadCommand } from './bindings/GamepadCommand';
import type { ScenarioCommand } from './bindings/ScenarioCommand';
import type { SystemCommand } from './bindings/SystemCommand';
import type { TextCommand } from './bindings/TextCommand';
import type { TransitionContainerCommand } from './bindings/TransitionContainerCommand';
import type { MakeNullOptional } from './utils';

type _Command =
  | AudioCommand
  | TextCommand
  | GamepadCommand
  | ScenarioCommand
  | SystemCommand
  | TransitionContainerCommand;

export type Command = MakeNullOptional<_Command>;

export type MaybePromise = any;

export * from './bindings/AudioCommand';
export * from './bindings/GamepadCommand';
export * from './bindings/ScenarioCommand';
export * from './bindings/SystemCommand';
export * from './bindings/TextCommand';
export * from './bindings/RetainMode';
export * from './bindings/AudioSettings';
export * from './bindings/EffectParams';
export * from './bindings/TransitionContainerCommand';
export * from './bindings/TransitionEffect';
export * from './bindings/WindowState';
