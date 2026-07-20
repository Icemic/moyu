import { createElement, type ReactNode } from 'react';
import type { MoyuClipAttributes, MoyuVBoxAttributes } from '../declaration';
import type { ScrollViewController } from '../hooks/useScrollView';
import { animated } from '../spring';

export interface ScrollViewProps {
  width: number;
  height: number;
  controller: ScrollViewController;
  children?: ReactNode;
  clipProps?: Omit<
    MoyuClipAttributes,
    'children' | 'height' | 'onTouchEnd' | 'onTouchMove' | 'onTouchStart' | 'onWheel' | 'width'
  >;
  contentProps?: Omit<MoyuVBoxAttributes, 'children' | 'onLayout' | 'y'>;
}

export function ScrollView({ width, height, controller, children, clipProps, contentProps }: ScrollViewProps) {
  return createElement(
    'clip',
    {
      ...clipProps,
      width,
      height,
      onWheel: controller.handleWheel,
      onTouchStart: controller.handleTouchStart,
      onTouchMove: controller.handleTouchMove,
      onTouchEnd: controller.handleTouchEnd,
      onTouchCancel: controller.handleTouchEnd,
    } satisfies MoyuClipAttributes,
    createElement(
      animated.vbox,
      {
        ...contentProps,
        y: controller.scrollOffset.to((value) => -value),
        onLayout: controller.handleContentLayout,
      },
      children,
    ),
  );
}
