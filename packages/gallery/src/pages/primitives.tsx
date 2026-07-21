import { useState } from 'react';
import { DemoChip, Panel } from '../components/chrome';
import { COLOR, ITEM_COLORS, TEXT, chipSprite } from '../theme';

function TextStylesPanel() {
  return (
    <Panel title="Text 文本样式" width={736} height={420} note="描边、阴影、颜色与 boxWidth 自动换行。">
      <vbox x={20} y={60} gap={22}>
        <text text="默认文本样式 Default 30px" fontSize={30} fillColor={COLOR.text} />
        <text
          text="描边与阴影 Stroke & Shadow"
          fontSize={34}
          fillColor={ITEM_COLORS[0]}
          stroke
          strokeColor="#101725"
          strokeWidth={3}
          shadow
          shadowColor="#000000aa"
          shadowOffsetX={4}
          shadowOffsetY={5}
          shadowBlur={4}
        />
        <hbox gap={18}>
          <text text="多彩" fontSize={28} fillColor={ITEM_COLORS[0]} />
          <text text="填充" fontSize={28} fillColor={ITEM_COLORS[1]} />
          <text text="颜色" fontSize={28} fillColor={ITEM_COLORS[2]} />
          <text text="展示" fontSize={28} fillColor={ITEM_COLORS[3]} />
        </hbox>
        <text
          text="固定宽度文本框：设定 boxWidth 后，这段文字会在到达宽度上限时自动换行，并可通过 lineHeight 控制行距。"
          fontSize={22}
          lineHeight={34}
          fillColor={COLOR.caption}
          boxWidth={660}
        />
      </vbox>
    </Panel>
  );
}

function TransformPanel() {
  const [transformed, setTransformed] = useState(false);

  return (
    <Panel
      title="Container 变换"
      width={736}
      height={420}
      note="x / scale / rotation 作用于整个子树；点击色块切换两组目标值。"
    >
      <clip x={20} y={60} width={696} height={260}>
        <container
          x={transformed ? 460 : 140}
          y={130}
          scale={transformed ? 1.3 : 1}
          rotation={transformed ? 0.15 : 0}
          interactive
          cursor="pointer"
          onClick={(event) => {
            event.stopPropagation();
            setTransformed((value) => !value);
          }}
        >
          <DemoChip label="点击切换变换" width={220} height={88} color={ITEM_COLORS[1]} />
        </container>
      </clip>
      <text
        {...TEXT.body}
        text={`当前目标：x=${transformed ? 460 : 140} · scale=${transformed ? 1.3 : 1.0} · rotation=${transformed ? 0.15 : 0}`}
        x={20}
        y={336}
      />
    </Panel>
  );
}

function SpritePanel() {
  return (
    <Panel title="Sprite 精灵" width={480} height={460} note="area 裁剪源图区域；nineslice 保持边角不变形地拉伸。">
      <sprite src="images/sample.png" area={[0, 0, 180, 120]} targetWidth={180} targetHeight={120} x={20} y={60} />
      <text {...TEXT.caption} text="area：左上象限 180×120" x={20} y={188} />
      <sprite src="images/sample.png" targetWidth={220} targetHeight={147} x={240} y={60} />
      <text {...TEXT.caption} text="完整原图 220×147" x={240} y={215} />
      <sprite {...chipSprite(ITEM_COLORS[0])} targetWidth={440} targetHeight={80} x={20} y={280}>
        <text text="nineslice 拉伸至 440×80" fontSize={20} fillColor="#ffffff" anchor={[0.5, 0.5]} pivot={[0.5, 0.5]} />
      </sprite>
      <text {...TEXT.caption} text="chip.png 九宫格：圆角与描边不随拉伸变形" x={20} y={372} />
    </Panel>
  );
}

function AnimationPanel() {
  return (
    <Panel title="Animation 帧动画" width={480} height={460} note="APNG 格式帧动画；tint 同样作用于动画帧。">
      <animation src="images/cursor.apng" format="apng" x={90} y={100} scale={4.5} />
      <animation src="images/cursor.apng" format="apng" tint={COLOR.accent} x={290} y={100} scale={4.5} />
      <text {...TEXT.caption} text="原色 32×32" x={110} y={280} />
      <text {...TEXT.caption} text="tint 金色" x={310} y={280} />
    </Panel>
  );
}

function ClipPanel() {
  return (
    <Panel title="Clip 裁剪" width={480} height={460} note="只有落在 440×260 视口内的内容会被绘制。">
      <clip x={20} y={70} width={440} height={260}>
        <text text="被裁剪的内容" fontSize={64} fillColor={ITEM_COLORS[3]} x={180} y={40} rotation={0.08} />
        <text
          text="这段文字超出了裁剪区域的右边界和下边界，超出的部分不会显示。"
          fontSize={26}
          fillColor={COLOR.text}
          boxWidth={560}
          x={60}
          y={160}
        />
      </clip>
    </Panel>
  );
}

export function PrimitivesPage() {
  return (
    <container>
      <hbox gap={32}>
        <TextStylesPanel />
        <TransformPanel />
      </hbox>
      <hbox gap={32} y={452}>
        <SpritePanel />
        <AnimationPanel />
        <ClipPanel />
      </hbox>
    </container>
  );
}
