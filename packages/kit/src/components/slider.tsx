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
  /** Called when the pointer presses the slider, before the drag starts. */
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
  /** Where the pointer was when the drag started, in stage coordinates. */
  startPointerX: number;
  startValue: number;
  /**
   * Identifier of the touch that started the drag, `undefined` when the mouse did. The
   * engine re-dispatches a mouse move every frame while the mouse pointer is over the
   * track, and the touch and mouse pointers are separate, so a drag only follows the
   * pointer device that started it.
   */
  pointerIdentifier?: number;
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
    const handleMouseUp = () => {
      if (dragStateRef.current?.pointerIdentifier === undefined) {
        endDragging(true);
      }
    };
    const handleTouchEnd = (event: TouchEvent) => {
      if (dragStateRef.current?.pointerIdentifier === event.identifier) {
        endDragging(true);
      }
    };
    const handleTouchCancel = (event: TouchEvent) => {
      if (dragStateRef.current?.pointerIdentifier === event.identifier) {
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

    onPress?.(event);
    if (event.defaultPrevented) {
      return;
    }

    const thumbPosition = currentValueRef.current * distance;
    const onThumb = event.offsetX >= thumbPosition && event.offsetX <= thumbPosition + thumb.targetWidth;
    let startValue = currentValueRef.current;

    // Pressing the bare track moves the thumb under the pointer and the drag continues
    // from there, so a tap jumps to the pressed position and a drag follows the pointer.
    if (!onThumb && distance > 0) {
      startValue = clampValue((event.offsetX - thumb.targetWidth / 2) / distance);
      setSliderValue(startValue);
    }

    dragStateRef.current = {
      // The movement is measured in stage coordinates: `offsetX` is relative to the node
      // the pointer is over, which changes when the pointer leaves the track.
      startPointerX: event.clientX,
      startValue,
      pointerIdentifier: 'identifier' in event ? event.identifier : undefined,
    };
    setDragging(true);
  };

  const moveDragging = (event: MouseEvent | TouchEvent) => {
    const dragState = dragStateRef.current;
    const eventIdentifier = 'identifier' in event ? event.identifier : undefined;
    if (dragState === null || dragState.pointerIdentifier !== eventIdentifier || distance === 0) {
      return;
    }
    event.stopPropagation();
    setSliderValue(dragState.startValue + (event.clientX - dragState.startPointerX) / distance);
  };

  const handleTouchCancel = (event: TouchEvent) => {
    if (dragStateRef.current?.pointerIdentifier === event.identifier) {
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
