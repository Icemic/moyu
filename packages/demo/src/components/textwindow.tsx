import React, { useEffect, useRef } from 'react';
import { animated, useSpring, Node } from '@hai/lib';
import { Button } from './button';

export interface TextWindowProps {
  onItemClicked?: (id: string) => void;
}

export function TextWindow(props: TextWindowProps) {
  const { onItemClicked } = props;
  const textWindowRef = useRef<Node>(null);

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
    onResolve: () => {
      textWindowRef.current?.executeCommand({ subCommand: 'setText', text: '这是一段测试文字' });
    },
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
      <text
        ref={textWindowRef}
        label="文本框内容"
        text={'这段文字会被打断aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'}
        fontSize={28}
        lineHeight={1.5}
        boxWidth={1280 - 50 * 2}
        boxHeight={200}
        fillColor={'white'}
        printMode="typewriter"
        printSpeed={20}
        stroke={true}
        strokeColor={'#232B6B'}
        x={60}
        y={70}
      />
      <Button fileName="btn_sys_01_hide" label="隐藏按钮" x={898} y={190} onClick={handleItemClick('hide')} />
      <Button fileName="btn_sys_01_Log" label="历史记录按钮" x={986} y={190} onClick={handleItemClick('log')} />
      <Button fileName="btn_sys_01_menu" label="菜单按钮" x={1098} y={190} onClick={handleItemClick('menu')} />
      <Button fileName="btn_sys_01_config" label="设置按钮" x={1190} y={190} onClick={handleItemClick('config')} />
    </animated.container>
  );
}
