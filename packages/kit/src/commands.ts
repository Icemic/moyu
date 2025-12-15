import { AudioCommand } from './bindings/AudioCommand';
import { GamepadCommand } from './bindings/GamepadCommand';
import { ScenarioCommand } from './bindings/ScenarioCommand';
import { SystemCommand } from './bindings/SystemCommand';
import { TextCommand } from './bindings/TextCommand';
import { MakeNullOptional } from './utils';

type _Command = AudioCommand | TextCommand | GamepadCommand | ScenarioCommand | SystemCommand;

export type Command = MakeNullOptional<_Command>;

export type MaybePromise = any;

export * from './bindings/AudioCommand';
export * from './bindings/GamepadCommand';
export * from './bindings/ScenarioCommand';
export * from './bindings/SystemCommand';
export * from './bindings/TextCommand';
export * from './bindings/AudioSettings';
export * from './bindings/EffectParams';
export * from './bindings/WindowState';
