import { Button } from '@momoyu-ink/kit';
import { useState } from 'react';
import { DemoChip, Panel, SectionTabs } from '../components/chrome';
import { BUTTON_SPRITE, BUTTON_TEXT_STYLE, COLOR, ITEM_COLORS, TEXT, ZONE_SPRITE } from '../theme';

type LayoutTab = 'basic' | 'alignment' | 'transform' | 'dynamic';
type JustifyContent = 'start' | 'center' | 'end' | 'space-between';
type AlignItems = 'start' | 'center' | 'end';

const JUSTIFY_VALUES: JustifyContent[] = ['start', 'center', 'end', 'space-between'];
const ALIGN_VALUES: AlignItems[] = ['start', 'center', 'end'];

const [BLUE, GREEN, RED, PURPLE, ORANGE] = ITEM_COLORS;

// ---------------------------------------------------------------------------
// Tab 1: auto sizing and nesting
// ---------------------------------------------------------------------------

function BasicTab() {
  return (
    <container>
      <Panel
        title="1. 自动 VBox"
        width={480}
        height={848}
        note="预期：左侧对齐；最宽项决定宽度；间距一致。"
      >
        <vbox padding={18} gap={14}>
          <DemoChip label="240 × 56" width={240} height={56} color={BLUE} />
          <DemoChip label="360 × 72" width={360} height={72} color={GREEN} />
          <text text="固有尺寸文本项" fontSize={26} fillColor={COLOR.text} />
          <DemoChip label="180 × 48" width={180} height={48} color={RED} />
        </vbox>
        <text {...TEXT.caption} text="padding = 18 · gap = 14" x={20} y={420} />
      </Panel>

      <Panel
        title="2. 自动 HBox"
        width={992}
        height={400}
        x={512}
        note="预期：垂直居中；paddingX/Y 覆盖 padding；所有直接子项参与测量。"
      >
        <hbox x={20} y={80} padding={8} paddingX={24} paddingY={16} gap={18} alignItems="center">
          <DemoChip label="150 × 54" width={150} height={54} color={BLUE} />
          <DemoChip label="220 × 90" width={220} height={90} color={GREEN} />
          <text text="文字固有尺寸" fontSize={24} fillColor={COLOR.text} />
          <DemoChip label="130 × 66" width={130} height={66} color={RED} />
        </hbox>
        <text {...TEXT.caption} text="padding = 8 · paddingX = 24 · paddingY = 16 · gap = 18" x={20} y={240} />
      </Panel>

      <Panel
        title="3. 嵌套：VBox → HBox → VBox"
        width={992}
        height={416}
        x={512}
        y={432}
        note="预期：嵌套尺寸逐层传递；内层两项右对齐；兄弟间无重叠。"
      >
        <vbox x={20} y={64} gap={16} padding={12}>
          <DemoChip label="外层 VBox" width={280} height={48} color={PURPLE} />
          <hbox gap={20} alignItems="center">
            <DemoChip label="左" width={150} height={78} color={BLUE} />
            <vbox gap={10} paddingX={14} paddingY={8} alignItems="end">
              <DemoChip label="内 VBox A" width={210} height={44} color={GREEN} />
              <DemoChip label="内 VBox B" width={150} height={58} color={RED} />
            </vbox>
            <DemoChip label="右" width={190} height={62} color={ORANGE} />
          </hbox>
        </vbox>
      </Panel>
    </container>
  );
}

// ---------------------------------------------------------------------------
// Tab 2: justifyContent / alignItems playground
// ---------------------------------------------------------------------------

function AlignmentTab() {
  const [justify, setJustify] = useState<JustifyContent>('start');
  const [align, setAlign] = useState<AlignItems>('start');

  return (
    <container>
      <hbox gap={16} alignItems="center">
        <Button
          sprite={{ ...BUTTON_SPRITE, targetWidth: 340, targetHeight: 56 }}
          text={`justifyContent: ${justify}`}
          textStyle={BUTTON_TEXT_STYLE}
          onPress={() => setJustify((current) => JUSTIFY_VALUES[(JUSTIFY_VALUES.indexOf(current) + 1) % JUSTIFY_VALUES.length])}
        />
        <Button
          sprite={{ ...BUTTON_SPRITE, targetWidth: 280, targetHeight: 56 }}
          text={`alignItems: ${align}`}
          textStyle={BUTTON_TEXT_STYLE}
          onPress={() => setAlign((current) => ALIGN_VALUES[(ALIGN_VALUES.indexOf(current) + 1) % ALIGN_VALUES.length])}
        />
        <text {...TEXT.caption} text="点击按钮循环切换；两块区域同步变化。" />
      </hbox>

      <Panel title="4. 固定 VBox：520 × 560" width={600} height={760} y={72} note="深色区域为容器的固定尺寸。">
        <sprite {...ZONE_SPRITE} targetWidth={520} targetHeight={560} x={40} y={64} />
        <vbox x={40} y={64} width={520} height={560} padding={20} gap={18} justifyContent={justify} alignItems={align}>
          <DemoChip label="260 × 54" width={260} height={54} color={BLUE} />
          <DemoChip label="390 × 72" width={390} height={72} color={GREEN} />
          <DemoChip label="180 × 62" width={180} height={62} color={RED} />
        </vbox>
      </Panel>

      <Panel
        title="5. 固定 HBox：780 × 420"
        width={872}
        height={760}
        x={632}
        y={72}
        note="space-between：间隔相等且不小于 gap = 20。"
      >
        <sprite {...ZONE_SPRITE} targetWidth={780} targetHeight={420} x={46} y={64} />
        <hbox x={46} y={64} width={780} height={420} padding={20} gap={20} justifyContent={justify} alignItems={align}>
          <DemoChip label="150 × 100" width={150} height={100} color={BLUE} />
          <DemoChip label="190 × 180" width={190} height={180} color={GREEN} />
          <DemoChip label="130 × 72" width={130} height={72} color={RED} />
        </hbox>
      </Panel>
    </container>
  );
}

// ---------------------------------------------------------------------------
// Tab 3: visual offsets vs. measurement
// ---------------------------------------------------------------------------

function TransformTab() {
  const [measured, setMeasured] = useState('等待测量');

  return (
    <container>
      <Panel
        title="6. 直接子项的 x/y 是视觉偏移"
        width={1504}
        height={380}
        note="预期：x = 45 的项目向右覆盖部分间隔，但兄弟项仍按原占位排列；anchor 被布局忽略。"
      >
        <hbox x={20} y={100} gap={24} alignItems="center">
          <DemoChip label="基准 A" width={190} height={70} color={BLUE} />
          <container x={45}>
            <DemoChip label="x = 45" width={190} height={70} color={GREEN} />
          </container>
          <DemoChip label="基准 B" width={190} height={70} color={RED} />
          <container anchor={[0.5, 0.5]}>
            <DemoChip label="anchor = .5" width={190} height={70} color={ORANGE} />
          </container>
          <DemoChip label="基准 C" width={190} height={70} color={PURPLE} />
        </hbox>
      </Panel>

      <Panel
        title="7. 变换与测量"
        width={740}
        height={436}
        y={412}
        note="预期：变换项按未变换尺寸占位；pivot 右对齐文字不扩大容器测量宽度。"
      >
        <hbox x={20} y={90} gap={36} alignItems="center">
          <DemoChip label="正常" width={170} height={80} color={BLUE} />
          <container scale={0.7} rotation={-0.12}>
            <DemoChip label="scale + rotation" width={220} height={100} color={GREEN} />
          </container>
          <container>
            <DemoChip label="" width={220} height={80} color={PURPLE} />
            <text text="right aligned" fontSize={20} fillColor="#ffffff" x={200} y={28} pivot={[1, 0]} />
          </container>
        </hbox>
      </Panel>

      <Panel
        title="8. onLayout 测量回读"
        width={732}
        height={436}
        x={772}
        y={412}
        note="预期：onLayout 报告的宽度 = 三个色块 + 两个间隔。"
      >
        <hbox
          x={20}
          y={90}
          gap={18}
          onLayout={(event) => setMeasured(`${event.width.toFixed(0)} × ${event.height.toFixed(0)}`)}
        >
          <DemoChip label="150" width={150} height={70} color={BLUE} />
          <DemoChip label="220" width={220} height={90} color={GREEN} />
          <DemoChip label="180" width={180} height={60} color={RED} />
        </hbox>
        <text {...TEXT.body} text={`HBox onLayout：${measured}`} x={20} y={230} />
        <text {...TEXT.caption} text="理论值：150 + 220 + 180 + 2 × 18 = 586" x={20} y={270} />
      </Panel>
    </container>
  );
}

// ---------------------------------------------------------------------------
// Tab 4: dynamic relayout
// ---------------------------------------------------------------------------

const DYNAMIC_LABELS = ['初始布局', 'B 变宽', 'B 隐藏但占位', '删除 B 并重排'] as const;

function DynamicTab() {
  const [mode, setMode] = useState(0);

  return (
    <Panel
      title="9. 动态尺寸 / 隐藏 / 删除 / 重排"
      width={1504}
      height={848}
      note="预期：每次点击同帧完成重排；visible = false 仍占位；删除后兄弟项递补到最前。"
    >
      <hbox x={20} y={64} gap={20} alignItems="center">
        <Button
          sprite={{ ...BUTTON_SPRITE, targetWidth: 320, targetHeight: 56 }}
          text={`下一状态：${DYNAMIC_LABELS[(mode + 1) % DYNAMIC_LABELS.length]}`}
          textStyle={BUTTON_TEXT_STYLE}
          onPress={() => setMode((value) => (value + 1) % DYNAMIC_LABELS.length)}
        />
        <text {...TEXT.body} text={`当前：${DYNAMIC_LABELS[mode]}`} fillColor={COLOR.accent} />
      </hbox>

      <sprite {...ZONE_SPRITE} targetWidth={960} targetHeight={220} x={20} y={170} />
      <hbox x={20} y={170} width={960} height={220} gap={18} padding={24} alignItems="center">
        {mode === 3 ? <DemoChip label="C" width={150} height={70} color={RED} /> : null}
        <DemoChip label="A" width={150} height={70} color={BLUE} />
        {mode !== 3 ? (
          <container visible={mode !== 2}>
            <DemoChip label="B" width={mode === 0 ? 150 : 260} height={70} color={GREEN} />
          </container>
        ) : null}
        {mode !== 3 ? <DemoChip label="C" width={150} height={70} color={RED} /> : null}
      </hbox>

      <vbox x={20} y={440} gap={12}>
        <text {...TEXT.body} text="状态说明" fillColor={COLOR.panelTitle} />
        <text {...TEXT.caption} text="· 初始布局：A、B、C 顺序排列，B 宽 150。" />
        <text {...TEXT.caption} text="· B 变宽：B 宽度变为 260，C 相应后移。" />
        <text {...TEXT.caption} text="· B 隐藏但占位：visible = false 不渲染，但布局位置保留。" />
        <text {...TEXT.caption} text="· 删除 B 并重排：B 从树中移除，C 移动到 A 之前。" />
      </vbox>
    </Panel>
  );
}

// ---------------------------------------------------------------------------

export function LayoutsPage() {
  const [tab, setTab] = useState<LayoutTab>('basic');

  return (
    <container>
      <SectionTabs
        value={tab}
        onChange={setTab}
        options={[
          { value: 'basic', label: '基础与嵌套' },
          { value: 'alignment', label: '对齐交互' },
          { value: 'transform', label: '变换与测量' },
          { value: 'dynamic', label: '动态重排' },
        ]}
      />
      <container y={64}>
        {tab === 'basic' ? <BasicTab /> : null}
        {tab === 'alignment' ? <AlignmentTab /> : null}
        {tab === 'transform' ? <TransformTab /> : null}
        {tab === 'dynamic' ? <DynamicTab /> : null}
      </container>
    </container>
  );
}
