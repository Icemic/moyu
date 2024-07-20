import React from 'react';
import { SpringValue, animated, useSpring, HaiTextAttribute } from '@hai/lib';

const LIST_TEXT_LAYOUT_STYLE: HaiTextAttribute = {
  direction: 'horizontal' as const,
  boxWidth: 200,
  boxHeight: 36,
  glyphGridSize: 24,
};

const LIST_TEXT_STYLE_DEFAULT: HaiTextAttribute = {
  fontSize: 24,
  lineHeight: 1.5,
  fillColor: 'rgba(255, 255, 255, 0.6)',
  indent: 0,
  // stroke: {},
  // shadow: {},
};

export interface ListButtonProps {
  label: string;
  title: string;
  index: number;
  style: Record<string, SpringValue<any>>;
  onClick?: (index: number) => void;
}

export function ListButton(props: ListButtonProps) {
  const { label, title, index, style, onClick } = props;

  const [springs, api] = useSpring(() => ({
    from: {
      opacity: 0,
      fillColor: 'rgba(255, 255, 255, 0.6)',
    },
  }));

  const handleEnter = () => {
    api.start({
      to: {
        opacity: 1,
        fillColor: 'rgba(255, 200, 80, 0.6)',
      },
    });
  };

  const handleLeave = () => {
    api.start({
      to: {
        opacity: 0,
        fillColor: 'rgba(255, 255, 255, 0.6)',
      },
    });
  };

  return (
    <animated.container label={`${label}-container`} {...style} y={index * 36}>
      <animated.sprite
        label={`${label}-底纹`}
        src="text_plate_01_transparent.png"
        anchor={[0.5, 0.0]}
        x={100}
        scaleX={4 / 15}
        scaleY={0.6}
        opacity={springs.opacity}
      />
      <animated.text
        label={`${label}-文本`}
        text={title}
        anchor={[0.5, 0.0]}
        x={100}
        {...LIST_TEXT_LAYOUT_STYLE}
        {...LIST_TEXT_STYLE_DEFAULT}
        fillColor={springs.fillColor}
        onMouseEnter={handleEnter}
        onMouseLeave={handleLeave}
        onTouchStart={handleEnter}
        onTouchEnd={handleLeave}
        onTouchCancel={handleLeave}
        onClick={() => onClick?.(index)}
        cursor={'pointer'}
      />
    </animated.container>
  );
}
