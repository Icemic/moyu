import { useEffect, useState } from 'react';
import { addEventListener, type MouseEvent, type TouchEvent } from '../events';
import type { MoyuNodeAttributes } from '../declaration';
import { mergeEvent } from '../utils';
import {
  type ControlSpriteProps,
  type ControlState,
  type ControlStateValue,
  type ControlTextStyle,
  resolveControlStateValue,
} from './control';

export type PressEvent = MouseEvent | TouchEvent;

export interface ButtonProps extends Omit<MoyuNodeAttributes, 'onClick'> {
  sprite: ControlSpriteProps;
  disabled?: boolean;
  lockOn?: Exclude<ControlState, 'disabled'>;
  onPress?: (event: PressEvent) => void;
  text?: string;
  textStyle?: ControlStateValue<ControlTextStyle>;
  textOffsetX?: number;
  textOffsetY?: number;
  textAlign?: 'left' | 'center' | 'right';
}

export function Button({
  sprite,
  disabled = false,
  lockOn,
  onPress,
  text,
  textStyle,
  textOffsetX,
  textOffsetY,
  textAlign = 'center',
  children,
  anchor,
  pivot = anchor,
  interactive,
  onMouseEnter,
  onMouseLeave,
  onMouseDown,
  onMouseUp,
  onTouchStart,
  onTouchEnd,
  onTouchCancel,
  ...containerProps
}: ButtonProps) {
  const [hovered, setHovered] = useState(false);
  const [pressed, setPressed] = useState(false);

  useEffect(() => {
    if (!pressed) {
      return;
    }
    const release = () => setPressed(false);
    const removeMouseUp = addEventListener('mouseup', release);
    const removeTouchEnd = addEventListener('touchend', release);
    const removeTouchCancel = addEventListener('touchcancel', release);
    return () => {
      removeMouseUp();
      removeTouchEnd();
      removeTouchCancel();
    };
  }, [pressed]);

  useEffect(() => {
    if (disabled) {
      setHovered(false);
      setPressed(false);
    }
  }, [disabled]);

  const state: ControlState = disabled ? 'disabled' : (lockOn ?? (pressed ? 'press' : hovered ? 'hover' : 'idle'));
  const { src, ...spriteProps } = sprite;
  const resolvedTextStyle = textStyle ? resolveControlStateValue(textStyle, state) : undefined;
  const textAnchor: [number, number] = textAlign === 'left' ? [0, 0.5] : textAlign === 'right' ? [1, 0.5] : [0.5, 0.5];

  return (
    <container
      {...containerProps}
      anchor={anchor}
      pivot={pivot}
      interactive={disabled ? false : interactive}
      onMouseEnter={mergeEvent(onMouseEnter, (event: MouseEvent) => {
        if (event.targetId === event.currentTargetId) {
          event.stopPropagation();
          return;
        }
        setHovered(true);
      })}
      onMouseLeave={mergeEvent(onMouseLeave, () => {
        setHovered(false);
        setPressed(false);
      })}
      onMouseDown={mergeEvent(onMouseDown, () => setPressed(true))}
      onMouseUp={mergeEvent(onMouseUp, (event: MouseEvent) => {
        if (pressed) {
          onPress?.(event);
        }
        setPressed(false);
        setHovered(true);
      })}
      onTouchStart={mergeEvent(onTouchStart, () => setPressed(true))}
      onTouchEnd={mergeEvent(onTouchEnd, (event: TouchEvent) => {
        if (pressed) {
          onPress?.(event);
        }
        setPressed(false);
        setHovered(true);
      })}
      onTouchCancel={mergeEvent(onTouchCancel, () => {
        setHovered(false);
        setPressed(false);
      })}
      onClick={(event: MouseEvent) => event.stopPropagation()}
    >
      <sprite {...spriteProps} src={resolveControlStateValue(src, state)} cursor="pointer">
        {text !== undefined ? (
          <text
            {...resolvedTextStyle}
            text={text}
            x={textOffsetX}
            y={textOffsetY}
            interactive={false}
            anchor={textAnchor}
            pivot={textAnchor}
          />
        ) : undefined}
        {children}
      </sprite>
    </container>
  );
}
