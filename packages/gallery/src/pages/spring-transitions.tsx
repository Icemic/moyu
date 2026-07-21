import { animated, Button, useFadeIn, useFadeOut, useSpring, useTransition } from '@momoyu-ink/kit';
import { useEffect, useState } from 'react';
import { DemoChip, Panel } from '../components/chrome';
import { BUTTON_SPRITE, BUTTON_TEXT_STYLE, COLOR, ITEM_COLORS, TEXT } from '../theme';

function UseSpringPanel() {
  const [moved, setMoved] = useState(false);
  const movement = useSpring({
    x: moved ? 620 : 80,
    rotation: moved ? 0.15 : -0.08,
    scale: moved ? 1.25 : 1,
    opacity: moved ? 0.65 : 1,
    config: { tension: 240, friction: 24 },
  });

  return (
    <Panel title="useSpring 属性动画" width={1504} height={320}>
      <clip x={20} y={16} width={1440} height={170}>
        <animated.container
          x={movement.x}
          y={40}
          rotation={movement.rotation}
          scale={movement.scale}
          opacity={movement.opacity}
          interactive
          cursor="pointer"
          onClick={(event) => {
            event.stopPropagation();
            setMoved((value) => !value);
          }}
        >
          <text text="SPRING" fontSize={66} fillColor={COLOR.accent} pivot={[0.5, 0.5]} />
          <text text="x / rotation / scale / opacity" fontSize={21} fillColor={COLOR.text} y={62} pivot={[0.5, 0.5]} />
        </animated.container>
      </clip>
      <text
        {...TEXT.body}
        text={`点击 SPRING 切换目标状态。\n当前目标：x=${moved ? 620 : 80} · rotation=${moved ? 0.15 : -0.08} · scale=${moved ? 1.25 : 1} · opacity=${
          moved ? 0.65 : 1
        }`}
        x={20}
        y={170}
      />
    </Panel>
  );
}

function UseTransitionPanel() {
  const [items, setItems] = useState([1, 2, 3]);
  const transitions = useTransition(items, {
    keys: (item) => item,
    from: { opacity: 0, x: 80, scale: 0.75 },
    enter: { opacity: 1, x: 0, scale: 1 },
    leave: { opacity: 0, x: 180, scale: 0.75 },
    config: { tension: 280, friction: 24 },
  });

  return (
    <Panel
      title="useTransition 进出场"
      width={1504}
      height={280}
      note="添加 / 移除 keyed 项目，from → enter → leave 自动补间。"
    >
      <hbox gap={16}>
        <Button
          sprite={{ ...BUTTON_SPRITE, targetWidth: 180, targetHeight: 48 }}
          text="添加"
          textStyle={BUTTON_TEXT_STYLE}
          onPress={() => setItems((current) => [...current, Math.max(0, ...current) + 1])}
        />
        <Button
          sprite={{ ...BUTTON_SPRITE, targetWidth: 180, targetHeight: 48 }}
          text="移除"
          textStyle={BUTTON_TEXT_STYLE}
          disabled={items.length === 0}
          opacity={items.length > 0 ? 1 : 0.35}
          onPress={() => setItems((current) => current.slice(0, -1))}
        />
      </hbox>
      <hbox x={20} y={80} gap={28} alignItems="center">
        {transitions((style, item) => (
          <animated.container key={item} x={style.x} opacity={style.opacity} scale={style.scale}>
            <DemoChip
              label={`项目 ${item}`}
              width={120}
              height={64}
              color={ITEM_COLORS[(item - 1) % ITEM_COLORS.length]}
            />
          </animated.container>
        ))}
      </hbox>
    </Panel>
  );
}

function FadeHelpersPanel() {
  const [fadeInStyle, fadeInApi, finishFadeIn] = useFadeIn(3000, true);
  const [fadeOutStyle, fadeOutApi, finishFadeOut] = useFadeOut(3000, true);

  useEffect(() => {
    void fadeInApi.start({ pause: false });
    void fadeOutApi.start({ pause: false });
  }, [fadeInApi, fadeOutApi]);

  const replayFades = () => {
    fadeInApi.set({ opacity: 0 });
    fadeOutApi.set({ opacity: 1 });

    setTimeout(() => {
      void fadeInApi.start({ to: { opacity: 1 }, pause: false, config: { duration: 3000 } });
      void fadeOutApi.start({ to: { opacity: 0 }, pause: false, config: { duration: 3000 } });
    }, 1000);
  };

  return (
    <Panel title="淡入淡出辅助" width={1504} height={300}>
      <hbox y={16} gap={16}>
        <Button
          sprite={{ ...BUTTON_SPRITE, targetWidth: 180, targetHeight: 48 }}
          text="重播"
          textStyle={BUTTON_TEXT_STYLE}
          onPress={replayFades}
        />
        <Button
          sprite={{ ...BUTTON_SPRITE, targetWidth: 180, targetHeight: 48 }}
          text="立即完成"
          textStyle={BUTTON_TEXT_STYLE}
          onPress={() => {
            finishFadeIn();
            finishFadeOut();
          }}
        />
      </hbox>
      <clip x={20} y={100} width={1440} height={150}>
        <hbox gap={32}>
          <animated.sprite
            src="images/chip.png"
            mode="nineslice"
            bounds={[0.3, 0.3, 0.3, 0.3]}
            tint={ITEM_COLORS[0]}
            targetWidth={220}
            targetHeight={110}
            opacity={fadeInStyle.opacity}
          >
            <text text="淡入 FADE IN" fontSize={24} fillColor={COLOR.caption} x={40} y={30} />
          </animated.sprite>
          <animated.text text="淡出 FADE OUT" fontSize={34} fillColor={COLOR.accent} opacity={fadeOutStyle.opacity} />
          <container>
            <text text="背景目标 BACKDROP" fontSize={48} fillColor={ITEM_COLORS[3]} />
            <animated.backdrop
              width={230}
              height={130}
              x={100}
              opacity={fadeInStyle.opacity}
              filters={[{ type: 'blur', radius: 6 }]}
            />
          </container>
        </hbox>
      </clip>
    </Panel>
  );
}

export function SpringTransitionsPage() {
  return (
    <vbox gap={32}>
      <UseSpringPanel />
      <UseTransitionPanel />
      <FadeHelpersPanel />
    </vbox>
  );
}
