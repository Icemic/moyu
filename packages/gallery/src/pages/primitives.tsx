import { useState } from 'react';
import { DemoChip, Panel } from '../components/chrome';
import { COLOR, ITEM_COLORS, TEXT, chipSprite } from '../theme';

function TextStylesPanel() {
  return (
    <Panel title="Text 文本样式" width={736} height={380} note="描边、阴影、颜色与 boxWidth 自动换行。">
      <vbox gap={22}>
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
      height={380}
      note="x / scale / rotation 作用于整个子树；点击色块切换两组目标值。"
    >
      <clip width={696} height={260}>
        <container
          label="testaaa"
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
    <Panel title="Sprite 精灵" width={1504} height={430} note="area 裁剪源图区域；nineslice 保持边角不变形地拉伸。">
      <hbox gap={32}>
        <vbox gap={16} alignItems="center">
          <text {...TEXT.caption} text="完整原图 250x240" />
          <sprite src="images/sample.png" />
        </vbox>

        <vbox gap={16} alignItems="center">
          <text {...TEXT.caption} text="area：从中间截取" />
          <sprite src="images/sample.png" area={[0.1, 0.2, 0.9, 0.8]} />
        </vbox>

        <vbox gap={16} alignItems="center">
          <text {...TEXT.caption} text="九宫格：圆角与描边不随拉伸变形，线条不模糊" />
          <sprite {...chipSprite(ITEM_COLORS[0])} targetWidth={440} targetHeight={80}>
            <text text="440×80" fontSize={20} fillColor="#ffffff" anchor={[0.5, 0.5]} pivot={[0.5, 0.5]} />
          </sprite>
          <sprite {...chipSprite(ITEM_COLORS[0])} targetWidth={200} targetHeight={100}>
            <text text="200×100" fontSize={20} fillColor="#ffffff" anchor={[0.5, 0.5]} pivot={[0.5, 0.5]} />
          </sprite>
        </vbox>
      </hbox>
    </Panel>
  );
}

function AnimationPanel() {
  return (
    <Panel title="Animation 帧动画" width={480} height={320} note="APNG 格式帧动画；tint 同样作用于动画帧。">
      <hbox gap={100} y={32} width={480} justifyContent="center">
        <vbox gap={32} alignItems="center">
          <animation src="images/cursor.png" format="apng" pivot={[0.5, 0.5]} scale={2} />
          <text {...TEXT.caption} text="原色" />
        </vbox>
        <vbox gap={32} alignItems="center">
          <animation src="images/cursor.png" format="apng" tint={COLOR.accent} pivot={[0.5, 0.5]} scale={2} />
          <text {...TEXT.caption} text="tint 金色" />
        </vbox>
      </hbox>
    </Panel>
  );
}

function ClipPanel() {
  return (
    <Panel title="Clip 裁剪" width={480} height={320} note="只有落在 440×260 视口内的内容会被绘制。">
      <clip width={440} height={260}>
        <text text="被裁剪的内容" fontSize={64} fillColor={ITEM_COLORS[3]} x={180} rotation={0.08} />
        <text
          text="这段文字超出了裁剪区域的右边界和下边界，超出的部分不会显示。"
          fontSize={22}
          fillColor={COLOR.text}
          boxWidth={440}
          y={120}
        />
      </clip>
    </Panel>
  );
}

export function PrimitivesPage() {
  return (
    <container>
      <vbox gap={32}>
        <hbox gap={32}>
          <TextStylesPanel />
          <TransformPanel />
        </hbox>
        <hbox gap={32}>
          <SpritePanel />
        </hbox>
        <hbox gap={32}>
          <AnimationPanel />
          <ClipPanel />
        </hbox>
      </vbox>
    </container>
  );
}
