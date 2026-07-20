import { useSpringValue, type SpringValue } from '@react-spring/core';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { LayoutEvent } from '../bindings/LayoutEvent';
import type { TouchEvent } from '../events/touch';
import type { WheelEvent } from '../events/wheel';

export interface UseScrollViewOptions {
  viewportHeight: number;
  initialPosition?: 'start' | 'end';
  wheelStep?: number;
}

export interface ScrollViewController {
  scrollOffset: SpringValue<number>;
  contentHeight: number;
  maxScroll: number;
  handleContentLayout: (event: LayoutEvent) => void;
  handleWheel: (event: WheelEvent) => void;
  handleTouchStart: (event: TouchEvent) => void;
  handleTouchMove: (event: TouchEvent) => void;
  handleTouchEnd: (event: TouchEvent) => void;
  scrollTo: (offset: number, immediate?: boolean) => void;
  scrollToRatio: (ratio: number, immediate?: boolean) => void;
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

function normalizeWheelDelta(event: WheelEvent, viewportHeight: number) {
  switch (event.deltaMode) {
    case 'line':
      return event.deltaY * 36;
    case 'page':
      return event.deltaY * viewportHeight * 0.85;
    case 'pixel':
    default:
      return event.deltaY;
  }
}

export function useScrollView({
  viewportHeight,
  initialPosition = 'start',
  wheelStep = 72,
}: UseScrollViewOptions): ScrollViewController {
  const [contentHeight, setContentHeight] = useState(0);
  const [hasContentLayout, setHasContentLayout] = useState(false);
  const scrollTargetRef = useRef(0);
  const didSetInitialPositionRef = useRef(false);
  const touchDraggingRef = useRef(false);
  const touchStartYRef = useRef(0);
  const touchStartOffsetRef = useRef(0);
  const scrollOffset = useSpringValue(0, {
    config: {
      tension: 320,
      friction: 32,
    },
  });
  const maxScroll = useMemo(() => Math.max(0, contentHeight - viewportHeight), [contentHeight, viewportHeight]);

  const scrollTo = useCallback(
    (nextOffset: number, immediate = false) => {
      const offset = clamp(nextOffset, 0, maxScroll);
      if (offset === scrollTargetRef.current && !immediate) {
        return;
      }

      scrollTargetRef.current = offset;
      scrollOffset.start(offset, { immediate });
    },
    [maxScroll, scrollOffset],
  );

  const scrollToRatio = useCallback(
    (ratio: number, immediate = false) => {
      scrollTo(clamp(ratio, 0, 1) * maxScroll, immediate);
    },
    [maxScroll, scrollTo],
  );

  const handleContentLayout = useCallback(
    (event: LayoutEvent) => {
      if (!didSetInitialPositionRef.current) {
        const initialOffset = initialPosition === 'end' ? Math.max(0, event.height - viewportHeight) : 0;
        didSetInitialPositionRef.current = true;
        scrollTargetRef.current = initialOffset;
        scrollOffset.start(initialOffset, { immediate: true });
      }

      setContentHeight(event.height);
      setHasContentLayout(true);
    },
    [initialPosition, scrollOffset, viewportHeight],
  );

  useEffect(() => {
    if (!hasContentLayout) {
      return;
    }

    scrollTo(scrollTargetRef.current, true);
  }, [hasContentLayout, maxScroll, scrollTo]);

  const handleWheel = useCallback(
    (event: WheelEvent) => {
      if (maxScroll <= 0) {
        return;
      }

      const delta = normalizeWheelDelta(event, viewportHeight);
      if (delta === 0) {
        return;
      }

      event.stopPropagation();
      const step = -Math.sign(delta) * Math.max(wheelStep, Math.min(Math.abs(delta), viewportHeight * 0.45));
      scrollTo(scrollTargetRef.current + step);
    },
    [maxScroll, scrollTo, viewportHeight, wheelStep],
  );

  const handleTouchStart = useCallback(
    (event: TouchEvent) => {
      if (maxScroll <= 0) {
        return;
      }

      event.stopPropagation();
      touchDraggingRef.current = true;
      touchStartYRef.current = event.clientY;
      touchStartOffsetRef.current = scrollTargetRef.current;
    },
    [maxScroll],
  );

  const handleTouchMove = useCallback(
    (event: TouchEvent) => {
      if (!touchDraggingRef.current) {
        return;
      }

      event.stopPropagation();
      scrollTo(touchStartOffsetRef.current + touchStartYRef.current - event.clientY, true);
    },
    [scrollTo],
  );

  const handleTouchEnd = useCallback((event: TouchEvent) => {
    if (!touchDraggingRef.current) {
      return;
    }

    event.stopPropagation();
    touchDraggingRef.current = false;
  }, []);

  return {
    scrollOffset,
    contentHeight,
    maxScroll,
    handleContentLayout,
    handleWheel,
    handleTouchStart,
    handleTouchMove,
    handleTouchEnd,
    scrollTo,
    scrollToRatio,
  };
}
