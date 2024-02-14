import React, { useEffect } from 'react';
import { animated, useSpring } from '../../../src/lib';
import { LayoutSytle, TextStyle } from '../../../src/declaration';
import { Button } from './button';

const LIST_TEXT_LAYOUT_STYLE: LayoutSytle = {
  direction: 'horizontal' as const,
  boxWidth: 200,
  boxHeight: 36,
  glyphGridSize: 24,
};

const LIST_TEXT_STYLE_DEFAULT: TextStyle = {
  fontSize: 24,
  lineHeight: 1.5,
  fillColor: 'rgba(255, 255, 255, 0.6)',
  indent: 0,
  // stroke: {},
  // shadow: {},
};

export interface TextWindowProps {
  onItemClicked?: (id: string) => void;
}

export function TextWindow(props: TextWindowProps) {
  const { onItemClicked } = props;

  const [springs, api] = useSpring(() => ({
    from: {
      opacity: 0,
      delta: 720 - 220 + 100,
    },
    to: {
      opacity: 1,
      delta: 720 - 220,
    },
    delay: 1000,
  }));

  useEffect(() => {
    api.start();
    return () => {
      api.stop();
    };
  }, []);

  const handleItemClick = (id: string) => () => {
    onItemClicked?.(id);
  };

  return (
    <animated.container label={`对话文本框`} anchor={[0.0, 0.0]} opacity={springs.opacity} x={0} y={springs.delta}>
      <sprite label="white" src="black.png" scaleX={1280} scaleY={200} y={26} opacity={0.8} />
      <sprite label="bg" src="window_02.png" y={26} />
      <sprite label="nametag" src="nametag.png" x={40} />
      <Button fileName="btn_sys_01_hide" label="隐藏按钮" x={898} y={190} onClick={handleItemClick('hide')} />
      <Button fileName="btn_sys_01_Log" label="历史记录按钮" x={986} y={190} onClick={handleItemClick('log')} />
      <Button fileName="btn_sys_01_menu" label="菜单按钮" x={1098} y={190} onClick={handleItemClick('menu')} />
      <Button fileName="btn_sys_01_config" label="设置按钮" x={1190} y={190} onClick={handleItemClick('config')} />
    </animated.container>
  );
}
