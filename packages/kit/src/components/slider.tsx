import { useCallback, useEffect, useRef, useState } from 'react';
import type { MoyuNodeAttributes } from '../declaration';
import { addEventListener, type MouseEvent, type TouchEvent } from '../events';
import { mergeEvent } from '../utils';
import { Button, type PressEvent } from './button';
import type { ControlSpriteProps } from './control';

export interface SliderTrackProps extends Omit<ControlSpriteProps, 'targetWidth'> {
  targetWidth: number;
}

export interface SliderThumbProps extends Omit<ControlSpriteProps, 'targetWidth'> {
  targetWidth: number;
}

export interface SliderProps extends Omit<MoyuNodeAttributes, 'onClick'> {
  value?: number;
  defaultValue?: number;
  onValueChange?: (value: number) => void;
  onValueCommit?: (value: number) => void;
  onPress?: (event: PressEvent) => void;
  disabled?: boolean;
  track: SliderTrackProps;
  thumb: SliderThumbProps;
}

function clampValue(value: number): number {
  if (Number.isNaN(value)) {
    return 0;
  }
  return Math.max(0, Math.min(1, value));
}

interface DragState {
  startPosition: number;
  startValue: number;
  touchIdentifier?: number;
  startedOnThumb: boolean;
  moved: boolean;
}

export function Slider({
  value,
  defaultValue = 0,
  onValueChange,
  onValueCommit,
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
  const [dragging, setDragging] = useState(false);
  const currentValue = clampValue(value ?? internalValue);
  const currentValueRef = useRef(currentValue);
  currentValueRef.current = currentValue;
  const dragStateRef = useRef<DragState | null>(null);
  const distance = Math.max(0, track.targetWidth - thumb.targetWidth);

  const setSliderValue = useCallback((nextValue: number) => {
    const clampedValue = clampValue(nextValue);
    currentValueRef.current = clampedValue;
    if (value === undefined) {
      setInternalValue(clampedValue);
    }
    onValueChange?.(clampedValue);
  }, [onValueChange, value]);

  const endDragging = useCallback((commit: boolean) => {
    if (dragStateRef.current === null) {
      return;
    }
    dragStateRef.current = null;
    setDragging(false);
    if (commit) {
      onValueCommit?.(currentValueRef.current);
    }
  }, [onValueCommit]);

  useEffect(() => {
    const handleMouseUp = () => endDragging(true);
    const handleTouchEnd = (event: TouchEvent) => {
      if (dragStateRef.current?.touchIdentifier === event.identifier) {
        endDragging(true);
      }
    };
    const handleTouchCancel = (event: TouchEvent) => {
      if (dragStateRef.current?.touchIdentifier === event.identifier) {
        endDragging(false);
      }
    };
    const removeMouseUp = addEventListener('mouseup', handleMouseUp);
    const removeTouchEnd = addEventListener('touchend', handleTouchEnd);
    const removeTouchCancel = addEventListener('touchcancel', handleTouchCancel);
    return () => {
      removeMouseUp();
      removeTouchEnd();
      removeTouchCancel();
    };
  }, [endDragging]);

  useEffect(() => {
    if (disabled) {
      endDragging(false);
    }
  }, [disabled, endDragging]);

  const startDragging = (event: MouseEvent | TouchEvent) => {
    event.stopPropagation();
    if (dragStateRef.current !== null) {
      return;
    }
    const thumbPosition = currentValueRef.current * distance;
    dragStateRef.current = {
      startPosition: event.offsetX,
      startValue: currentValueRef.current,
      touchIdentifier: 'identifier' in event ? event.identifier : undefined,
      startedOnThumb: event.offsetX >= thumbPosition && event.offsetX <= thumbPosition + thumb.targetWidth,
      moved: false,
    };
    setDragging(true);
  };

  const moveDragging = (event: MouseEvent | TouchEvent) => {
    const dragState = dragStateRef.current;
    if (
      dragState === null ||
      ('identifier' in event && dragState.touchIdentifier !== event.identifier) ||
      distance === 0
    ) {
      return;
    }
    event.stopPropagation();
    const delta = event.offsetX - dragState.startPosition;
    if (delta !== 0) {
      dragState.moved = true;
    }
    setSliderValue(dragState.startValue + delta / distance);
  };

  const handlePress = (event: PressEvent) => {
    const dragState = dragStateRef.current;
    if (dragState === null || ('identifier' in event && dragState.touchIdentifier !== event.identifier)) {
      return;
    }
    onPress?.(event);
    if (event.defaultPrevented) {
      endDragging(false);
      return;
    }
    if (!dragState.moved && !dragState.startedOnThumb && distance > 0) {
      setSliderValue((event.offsetX - thumb.targetWidth / 2) / distance);
    }
    endDragging(true);
  };

  const handleTouchCancel = (event: TouchEvent) => {
    if (dragStateRef.current?.touchIdentifier === event.identifier) {
      endDragging(false);
    }
  };

  return (
    <Button
      {...containerProps}
      anchor={anchor}
      pivot={pivot}
      interactive={interactive}
      disabled={disabled}
      lockOn={dragging ? 'press' : undefined}
      sprite={{
        ...track,
        pivot: [0, 0.5],
        y: (track.targetHeight ?? 0) / 2,
      }}
      onPress={handlePress}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onMouseDown={mergeEvent(onMouseDown, startDragging)}
      onMouseUp={onMouseUp}
      onMouseMove={mergeEvent(onMouseMove, moveDragging)}
      onTouchStart={mergeEvent(onTouchStart, startDragging)}
      onTouchMove={mergeEvent(onTouchMove, moveDragging)}
      onTouchEnd={onTouchEnd}
      onTouchCancel={mergeEvent(onTouchCancel, handleTouchCancel)}
    >
      <Button
        sprite={{
          ...thumb,
          anchor: [0, 0.5],
          pivot: [0, 0.5],
          x: 0,
          interactive: false,
        }}
        lockOn={dragging ? 'press' : undefined}
        x={currentValue * distance}
        anchor={[0, 0.5]}
        pivot={[0, 0.5]}
        interactive={false}
      />
    </Button>
  );
}
