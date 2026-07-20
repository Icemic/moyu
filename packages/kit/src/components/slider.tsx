import { useCallback, useEffect, useRef, useState } from 'react';
import type { SpriteProps } from '../bindings/SpriteProps';
import type { MoyuNodeAttributes } from '../declaration';
import { addEventListener, type MouseEvent, type TouchEvent } from '../events';
import { mergeEvent } from '../utils';
import type { PressEvent } from './button';
import { type ControlState, type ControlStateValue, resolveControlStateValue } from './control';

export interface SliderTrackProps extends Omit<SpriteProps, 'src' | 'targetWidth'> {
  src: ControlStateValue<string>;
  targetWidth: number;
}

export interface SliderThumbProps extends Omit<SpriteProps, 'src' | 'targetWidth'> {
  src: ControlStateValue<string>;
  targetWidth: number;
}

export interface SliderProps extends Omit<MoyuNodeAttributes, 'onClick'> {
  value?: number;
  defaultValue?: number;
  onValueChange?: (value: number) => void;
  onPress?: (event: PressEvent) => void;
  disabled?: boolean;
  track: SliderTrackProps;
  thumb: SliderThumbProps;
}

function clampValue(value: number): number {
  return Math.max(0, Math.min(1, value));
}

export function Slider({
  value,
  defaultValue = 0,
  onValueChange,
  onPress,
  disabled = false,
  track,
  thumb,
  anchor,
  pivot = anchor,
  interactive,
  onMouseEnter,
  onMouseLeave,
  onMouseDown,
  onMouseUp,
  onMouseMove,
  onTouchStart,
  onTouchMove,
  onTouchEnd,
  onTouchCancel,
  ...containerProps
}: SliderProps) {
  const [internalValue, setInternalValue] = useState(() => clampValue(defaultValue));
  const [hovered, setHovered] = useState(false);
  const [dragging, setDragging] = useState(false);
  const draggingRef = useRef(false);
  const currentValue = clampValue(value ?? internalValue);
  const distance = Math.max(0, track.targetWidth - thumb.targetWidth);
  const state: ControlState = disabled ? 'disabled' : dragging ? 'press' : hovered ? 'hover' : 'idle';
  const { src: trackSrc, ...trackProps } = track;
  const { src: thumbSrc, ...thumbProps } = thumb;

  const updateValue = (event: MouseEvent | TouchEvent) => {
    event.stopPropagation();
    if (distance === 0) {
      return;
    }
    const nextValue = clampValue((event.offsetX - thumb.targetWidth / 2) / distance);
    if (value === undefined) {
      setInternalValue(nextValue);
    }
    onValueChange?.(nextValue);
  };

  const endDragging = useCallback(() => {
    draggingRef.current = false;
    setDragging(false);
  }, []);

  useEffect(() => {
    const handleEnd = () => {
      if (draggingRef.current) {
        endDragging();
      }
    };
    const removeMouseUp = addEventListener('mouseup', handleEnd);
    const removeTouchEnd = addEventListener('touchend', handleEnd);
    const removeTouchCancel = addEventListener('touchcancel', handleEnd);
    return () => {
      removeMouseUp();
      removeTouchEnd();
      removeTouchCancel();
    };
  }, [endDragging]);

  useEffect(() => {
    if (disabled) {
      endDragging();
      setHovered(false);
    }
  }, [disabled, endDragging]);

  const startDragging = (event: MouseEvent | TouchEvent) => {
    onPress?.(event);
    if (event.defaultPrevented) {
      return;
    }
    draggingRef.current = true;
    setDragging(true);
    updateValue(event);
  };

  return (
    <container
      {...containerProps}
      anchor={anchor}
      pivot={pivot}
      interactive={disabled ? false : interactive}
      onMouseEnter={mergeEvent(onMouseEnter, () => setHovered(true))}
      onMouseLeave={mergeEvent(onMouseLeave, () => {
        setHovered(false);
        if (!draggingRef.current) {
          setDragging(false);
        }
      })}
      onMouseDown={mergeEvent(onMouseDown, startDragging)}
      onMouseUp={mergeEvent(onMouseUp, endDragging)}
      onMouseMove={mergeEvent(onMouseMove, (event: MouseEvent) => {
        if (draggingRef.current) {
          updateValue(event);
        }
      })}
      onTouchStart={mergeEvent(onTouchStart, startDragging)}
      onTouchMove={mergeEvent(onTouchMove, (event: TouchEvent) => {
        if (draggingRef.current) {
          updateValue(event);
        }
      })}
      onTouchEnd={mergeEvent(onTouchEnd, endDragging)}
      onTouchCancel={mergeEvent(onTouchCancel, endDragging)}
      onClick={(event: MouseEvent) => event.stopPropagation()}
    >
      <sprite {...trackProps} src={resolveControlStateValue(trackSrc, state)} cursor="pointer">
        <sprite
          {...thumbProps}
          src={resolveControlStateValue(thumbSrc, state)}
          anchor={[0, 0.5]}
          pivot={[0, 0.5]}
          x={currentValue * distance}
          interactive={false}
        />
      </sprite>
    </container>
  );
}
