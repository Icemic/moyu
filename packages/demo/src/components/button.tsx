import React, { useEffect, useState } from 'react';
import { animated, useSpring, HaiNodeAttributes, HaiEvent } from '@hai/lib';

export interface ButtonProps extends HaiNodeAttributes {
  fileName: string;
  label?: string;
  onClick?: () => void;
}

export function Button(props: ButtonProps) {
  const { fileName, label, onClick, anchor, ...restProps } = props;

  const [pressed, setPressed] = useState(false);

  const [springs, api] = useSpring(() => ({
    from: {
      idle_opacity: 1,
      hover_opacity: 0,
      visible: true,
      click_opacity: 0,
    },
  }));

  const handleEnter = (evt: HaiEvent) => {
    console.log('enter', evt.target_label, evt.current_target_label);
    if (evt.target_id === evt.current_target_id) {
      evt.stopPropagation();
      return;
    }

    api.start({
      to: {
        idle_opacity: 0,
        hover_opacity: 1,
        click_opacity: +pressed,
        visible: !pressed,
      },
    });
  };

  const handleLeave = () => {
    api.start({
      to: {
        idle_opacity: 1,
        hover_opacity: 0,
        click_opacity: 0,
        visible: true,
      },
    });
  };

  const handleMouseDown = () => {
    setPressed(true);
    api.start({
      to: {
        click_opacity: 1,
        visible: false,
      },
      config: {
        duration: 30,
      },
    });
  };

  const handleMouseUp = () => {
    setPressed(false);
    api.start({
      to: {
        click_opacity: 0,
        visible: true,
      },
      config: {
        duration: 30,
      },
    });
  };

  return (
    <container
      label={label}
      {...restProps}
      onMouseEnter={handleEnter}
      onMouseLeave={handleLeave}
      onMouseDown={handleMouseDown}
      onMouseUp={handleMouseUp}
      onTouchStart={handleMouseDown}
      onTouchEnd={handleMouseUp}
      onTouchCancel={handleMouseUp}
      onClick={onClick}
    >
      <animated.sprite
        label="button_idle"
        src={`${fileName}.png`}
        visible={springs.visible}
        opacity={springs.idle_opacity}
        anchor={anchor}
      />
      <animated.sprite
        label="button_hover"
        src={`${fileName}_hover.png`}
        visible={springs.visible}
        opacity={springs.hover_opacity}
        anchor={anchor}
      />
      <animated.sprite
        label="button_click"
        src={`${fileName}_click.png`}
        opacity={springs.click_opacity}
        anchor={anchor}
        cursor={'pointer'}
      />
    </container>
  );
}
