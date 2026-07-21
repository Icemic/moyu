import { animated, Button, Slider, useSpring } from '@momoyu-ink/kit';
import { useState } from 'react';
import { DemoChip, Panel } from '../components/chrome';
import { BUTTON_SPRITE, BUTTON_TEXT_STYLE, COLOR, ITEM_COLORS, SLIDER_THUMB, SLIDER_TRACK, TEXT } from '../theme';

const GROUP_LABEL = { fontSize: 18, fillColor: COLOR.dim } as const;

function ConsolePanel({
  blurred,
  onToggle,
  blurValue,
  onBlurChange,
  blurRadius,
  saturation,
  onSaturationChange,
}: {
  blurred: boolean;
  onToggle: () => void;
  blurValue: number;
  onBlurChange: (value: number) => void;
  blurRadius: number;
  saturation: number;
  onSaturationChange: (value: number) => void;
}) {
  return (
    <Panel title="控制台" width={400} height={760} note="滑块实时调整右侧场景的滤镜参数。">
      <vbox x={20} y={60} gap={24}>
        <text {...GROUP_LABEL} text="开关 Toggle" />
        <Button
          sprite={{ ...BUTTON_SPRITE, targetWidth: 320, targetHeight: 56 }}
          text={blurred ? '禁用背景滤镜' : '启用背景滤镜'}
          textStyle={BUTTON_TEXT_STYLE}
          onPress={onToggle}
        />
        <text {...GROUP_LABEL} text="模糊半径 Blur" />
        <Slider
          value={blurValue}
          onValueChange={onBlurChange}
          track={{ ...SLIDER_TRACK, targetWidth: 320 }}
          thumb={SLIDER_THUMB}
        />
        <text {...TEXT.body} text={`模糊半径 ${blurRadius}`} />
        <text {...GROUP_LABEL} text="饱和度 Saturation" />
        <Slider
          value={saturation}
          onValueChange={onSaturationChange}
          track={{ ...SLIDER_TRACK, targetWidth: 320 }}
          thumb={SLIDER_THUMB}
        />
        <text {...TEXT.body} text={`饱和度 ${saturation.toFixed(2)}`} />
        <text
          {...TEXT.caption}
          text="Backdrop 捕获其身后已绘制的画面并应用滤镜；在它之后绘制的内容不受影响。"
          boxWidth={360}
          lineHeight={28}
        />
      </vbox>
    </Panel>
  );
}

export function BackdropsPage() {
  const [blurred, setBlurred] = useState(true);
  const [blurValue, setBlurValue] = useState(0.5);
  const [saturation, setSaturation] = useState(0.5);
  const blurRadius = Math.round(2 + blurValue * 14);
  const backdropStyle = useSpring({
    opacity: blurred ? 0.96 : 0,
    config: { duration: 300 },
  });

  return (
    <container>
      <ConsolePanel
        blurred={blurred}
        onToggle={() => setBlurred((value) => !value)}
        blurValue={blurValue}
        onBlurChange={setBlurValue}
        blurRadius={blurRadius}
        saturation={saturation}
        onSaturationChange={setSaturation}
      />

      <Panel title="演示场景" width={1072} height={760} x={432} note="模糊与饱和度实时作用于捕获画面。">
        {/* Decorative backdrop content: drawn first, so it gets captured and filtered. */}
        <text text="BACKGROUND" fontSize={92} fillColor={ITEM_COLORS[0]} x={40} y={70} rotation={-0.06} />
        <text text="LAYER 01" fontSize={72} fillColor={ITEM_COLORS[2]} x={640} y={150} rotation={0.08} />
        <text text="渲染管线" fontSize={64} fillColor={ITEM_COLORS[1]} x={180} y={340} rotation={0.03} />
        <text text="背景捕获区域" fontSize={70} fillColor={COLOR.text} x={400} y={560} rotation={-0.04} />

        <animated.backdrop
          width={860}
          height={440}
          x={106}
          y={180}
          filters={[
            { type: 'blur', radius: blurRadius },
            { type: 'saturation', amount: saturation },
          ]}
          opacity={backdropStyle.opacity}
        />

        {/* Foreground content: drawn after the backdrop, so it stays sharp. */}
        <vbox x={200} y={360} gap={18}>
          <text text="backdrop 之后绘制的内容保持清晰" fontSize={28} fillColor={COLOR.pageTitle} />
          <DemoChip label="前景内容" width={160} height={48} color={ITEM_COLORS[4]} />
        </vbox>
      </Panel>
    </container>
  );
}
