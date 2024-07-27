import React from 'react';
import { CONTENT_HEIGHT, CONTENT_WIDTH, NORMAL_TEXT_STYLE, SIDEBAR_WIDTH } from '../constants';
import { HaiEvent, HaiTextAttribute } from '@hai/lib';

const BOX_X = CONTENT_WIDTH * 0.05;
const BOX_Y = 240;

export function MouseAndTouch() {
  const [location, setLocation] = React.useState({ x: 0, y: 0, clientX: 0, clientY: 0, screenX: 0, screenY: 0 });
  const [clickLocation, setClickLocation] = React.useState({ x: 0, y: 0 });

  const handleLocation = (event: HaiEvent) => {
    setLocation({
      x: (event.clientX ?? 0) - BOX_X - SIDEBAR_WIDTH,
      y: (event.clientY ?? 0) - BOX_Y,
      clientX: event.clientX ?? 0,
      clientY: event.clientY ?? 0,
      screenX: event.screenX ?? 0,
      screenY: event.screenY ?? 0,
    });
    setClickLocation({
      x: (event.clientX ?? 0) - BOX_X - SIDEBAR_WIDTH,
      y: (event.clientY ?? 0) - BOX_Y,
    });
    event.preventDefault();
  };

  const handleClick = (event: HaiEvent) => {
    setClickLocation({
      x: (event.clientX ?? 0) - BOX_X - SIDEBAR_WIDTH,
      y: (event.clientY ?? 0) - BOX_Y,
    });
    event.preventDefault();
  };

  return (
    <container>
      <sprite src="mask.png" x={CONTENT_WIDTH * 0.05} y={20} scaleX={CONTENT_WIDTH * 0.9} scaleY={200} />
      <text
        text={`x: ${location.x} y: ${location.y}\nclientX: ${location.clientX} clientY: ${location.clientY}\nscreenX: ${location.screenX} screenY: ${location.screenY}`.trim()}
        x={CONTENT_WIDTH * 0.05 + 10}
        y={30}
        boxWidth={CONTENT_WIDTH * 0.9}
        boxHeight={110}
        {...NORMAL_TEXT_STYLE}
      />

      <sprite
        src="mask.png"
        x={BOX_X}
        y={BOX_Y}
        scaleX={CONTENT_WIDTH * 0.9}
        scaleY={460}
        onMouseMove={handleLocation}
        onTouchStart={handleLocation}
        onTouchMove={handleLocation}
        onTouchEnd={handleClick}
        onMouseDown={handleClick}
        cursor="crosshair"
      />
      <sprite
        src="white.png"
        x={clickLocation.x + BOX_X}
        y={clickLocation.y + BOX_Y}
        scaleX={10}
        scaleY={10}
        opacity={0.6}
        anchor={[0.5, 0.5]}
        cursor="crosshair"
      />
    </container>
  );
}
