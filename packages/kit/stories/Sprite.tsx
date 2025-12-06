import React from 'react';
import { Cursor } from '../src/declaration';

export interface SpriteProps {
  /** Image source path (relative to assets/) */
  src: string;
  /** X position */
  x?: number;
  /** Y position */
  y?: number;
  /** Anchor point (default: [0, 0]) */
  anchor?: [number, number];
  /** Scale (uniform) */
  scale?: number;
  /** Scale X */
  scaleX?: number;
  /** Scale Y */
  scaleY?: number;
  /** Rotation in degrees */
  rotation?: number;
  /** Visibility */
  visible?: boolean;
  /** Tint color */
  tint?: string;
  /** Opacity (0-1) */
  opacity?: number;
  /** Interactive (enables mouse events) */
  interactive?: boolean;
  /** Cursor style when hovering */
  cursor?: Cursor;
  /** Click event handler */
  onClick?: () => void;
  /** Mouse enter event handler */
  onMouseEnter?: () => void;
  /** Mouse leave event handler */
  onMouseLeave?: () => void;
}

/**
 * Sprite Component - displays images with transform and interaction support
 * Uses the <sprite> element from @momoyu-ink/kit
 */
export const Sprite = ({
  src,
  x = 0,
  y = 0,
  anchor = [0, 0],
  scale,
  scaleX,
  scaleY,
  rotation = 0,
  visible = true,
  tint,
  opacity = 1,
  interactive = false,
  cursor = 'pointer',
  onClick,
  onMouseEnter,
  onMouseLeave,
}: SpriteProps) => {
  return (
    <sprite
      src={src}
      x={x}
      y={y}
      anchor={anchor}
      scale={scale}
      scaleX={scaleX}
      scaleY={scaleY}
      rotation={rotation}
      visible={visible}
      tint={tint}
      opacity={opacity}
      interactive={interactive}
      cursor={cursor}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    />
  );
};
