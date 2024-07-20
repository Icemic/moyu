import React, { useEffect } from 'react';
import { animated, useSpring, useTransition, HaiNodeAttributes } from '@hai/lib';
import { Button } from './button';

export interface DialogProps extends HaiNodeAttributes {
  title: string;
  content: string;
  mode: 'alert' | 'confirm';
  show: boolean;
  onConfirm?: (yes?: boolean) => void;
}

export function Dialog(props: DialogProps) {
  const { title, content, mode, show, onConfirm } = props;

  console.log('dialog show', show);

  const transitions = useTransition(show ? [0] : [], {
    keys: (item) => item,
    from: {
      opacity: 0,
      scale: 0.3,
    },
    enter: {
      opacity: 1,
      scale: 1,
    },
    leave: {
      opacity: 0,
      scale: 0.8,
    },
    config: {
      mass: 0.5,
      tension: 280,
      friction: 12,
    },
  });

  return transitions((style, _) => (
    <animated.container anchor={[0.5, 0.5]} x={640} y={360} {...style}>
      <sprite label="对话框" src="dialog/dialog_bg.png" anchor={[0.5, 0]} y={-155} opacity={0.98} scale={0.5} />
      <text
        label="对话框标题"
        text={title}
        anchor={[0.5, 0]}
        fontSize={32}
        lineHeight={1.5}
        fillColor={'white'}
        x={0}
        y={15 - 155}
      />
      <text
        label="对话框内容"
        text={content}
        anchor={[0.5, 0]}
        fontSize={24}
        lineHeight={1.5}
        fillColor={'white'}
        x={0}
        y={100 - 155}
      />
      {mode === 'confirm' && (
        <Button
          fileName="dialog/btn_ok"
          label="对话框确认按钮"
          anchor={[0.5, 0]}
          x={0}
          y={230 - 155}
          onClick={() => onConfirm?.()}
        />
      )}
      {mode === 'alert' && (
        <>
          <Button
            fileName="dialog/btn_yes"
            label="对话框同意按钮"
            anchor={[0.5, 0]}
            x={-120}
            y={230 - 155}
            onClick={() => onConfirm?.(true)}
          />
          <Button
            fileName="dialog/btn_no"
            label="对话框拒绝按钮"
            anchor={[0.5, 0]}
            x={120}
            y={230 - 155}
            onClick={() => onConfirm?.(false)}
          />
        </>
      )}
    </animated.container>
  ));
}
