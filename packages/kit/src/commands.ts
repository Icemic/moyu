import type { AudioCommand } from './bindings/AudioCommand';
import type { GamepadCommand } from './bindings/GamepadCommand';
import type { ScenarioCommand } from './bindings/ScenarioCommand';
import type { ShaderCommand } from './bindings/ShaderCommand';
import type { SystemCommand } from './bindings/SystemCommand';
import type { TextCommand } from './bindings/TextCommand';
import type { MakeNullOptional } from './utils';

type _Command =
  | AudioCommand
  | TextCommand
  | GamepadCommand
  | ScenarioCommand
  | ShaderCommand
  | SystemCommand;

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
export * from './bindings/ShaderBuiltin';
export * from './bindings/ShaderBuiltinName';
export * from './bindings/ShaderCommand';
export * from './bindings/ShaderParam';
export * from './bindings/ShaderParamType';
export * from './bindings/ShaderSource';
export * from './bindings/ShaderTimeControl';
export * from './bindings/WindowState';
