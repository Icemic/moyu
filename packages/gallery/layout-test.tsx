import { Button, getStageSize, useNavigation } from '@momoyu-ink/kit';
import { useState, type ReactNode } from 'react';
import { uiActions } from '../state/ui';

const BUTTON_FILES = ['ui/selection.png', 'ui/selection_hover.png', 'ui/selection_press.png'] as const;
const BUTTON_BOUNDS: [number, number, number, number] = [0.2, 0.2, 0.2, 0.2];
const PANEL_SRC = 'ui/selection.png';
const PANEL_BOUNDS: [number, number, number, number] = [0.2, 0.2, 0.2, 0.2];

type TestPage = 'basic' | 'alignment' | 'transform' | 'compatibility';
type JustifyContent = 'start' | 'center' | 'end' | 'space-between';
type AlignItems = 'start' | 'center' | 'end';

const TEST_PAGES: Array<{ key: TestPage; label: string }> = [
  { key: 'basic', label: '基础与嵌套' },
  { key: 'alignment', label: '主轴与交叉轴' },
  { key: 'transform', label: '偏移与变换' },
  { key: 'compatibility', label: '现有页面回归' },
];

const JUSTIFY_VALUES: JustifyContent[] = ['start', 'center', 'end', 'space-between'];
const ALIGN_VALUES: AlignItems[] = ['start', 'center', 'end'];

function TestItem({ text, width, height, tint }: { text: string; width: number; height: number; tint: string }) {
  return (
    <Button
      sprite={{
        src: BUTTON_FILES,
        mode: 'nineslice',
        bounds: BUTTON_BOUNDS,
        targetWidth: width,
        targetHeight: height,
        tint,
      }}
      text={text}
      textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
    />
  );
}

function TestPanel({
  x,
  y,
  width,
  height,
  title,
  children,
}: {
  x: number;
  y: number;
  width: number;
  height: number;
  title: string;
  children: ReactNode;
}) {
  return (
    <sprite
      src={PANEL_SRC}
      mode="nineslice"
      bounds={PANEL_BOUNDS}
      targetWidth={width}
      targetHeight={height}
      x={x}
      y={y}
      tint="#20283d"
    >
      <text text={title} fontSize={25} fillColor="#f7d98b" x={24} y={18} />
      {children}
    </sprite>
  );
}

function BasicTests() {
  return (
    <>
      <TestPanel x={70} y={190} width={550} height={770} title="1. Auto VBox：padding=18, gap=14">
        <vbox x={28} y={72} padding={18} gap={14}>
          <TestItem text="240 × 56" width={240} height={56} tint="#4f75c9" />
          <TestItem text="360 × 72" width={360} height={72} tint="#5f9b72" />
          <text text="Intrinsic text item" fontSize={28} fillColor="#ffffff" />
          <TestItem text="180 × 48" width={180} height={48} tint="#a66b75" />
        </vbox>
        <text text="预期：左侧对齐；最大项决定宽度；间距一致。" fontSize={20} fillColor="#b8c2dc" x={28} y={650} />
      </TestPanel>

      <TestPanel x={685} y={190} width={1165} height={350} title="2. Auto HBox：paddingX=24, paddingY=16, gap=18">
        <hbox x={28} y={85} padding={8} paddingX={24} paddingY={16} gap={18} alignItems="center">
          <TestItem text="150 × 54" width={150} height={54} tint="#4f75c9" />
          <TestItem text="220 × 90" width={220} height={90} tint="#5f9b72" />
          <text text="文字固有尺寸" fontSize={27} fillColor="#ffffff" />
          <TestItem text="130 × 66" width={130} height={66} tint="#a66b75" />
        </hbox>
        <text
          text="预期：垂直居中；paddingX/Y 覆盖 padding；所有直接子项参与测量。"
          fontSize={20}
          fillColor="#b8c2dc"
          x={28}
          y={275}
        />
      </TestPanel>

      <TestPanel x={685} y={590} width={1165} height={370} title="3. Nested：VBox → HBox → VBox">
        <vbox x={28} y={78} gap={16} padding={12}>
          <TestItem text="外层 VBox" width={280} height={48} tint="#7d68b5" />
          <hbox gap={20} alignItems="center">
            <TestItem text="左" width={150} height={78} tint="#4f75c9" />
            <vbox gap={10} paddingX={14} paddingY={8} alignItems="end">
              <TestItem text="内 VBox A" width={210} height={44} tint="#5f9b72" />
              <TestItem text="内 VBox B" width={150} height={58} tint="#a66b75" />
            </vbox>
            <TestItem text="右" width={190} height={62} tint="#b18448" />
          </hbox>
        </vbox>
        <text
          text="预期：嵌套尺寸逐层传递；内层两个项目右对齐；兄弟间无重叠。"
          fontSize={20}
          fillColor="#b8c2dc"
          x={28}
          y={300}
        />
      </TestPanel>
    </>
  );
}

function AlignmentTests() {
  const [justify, setJustify] = useState<JustifyContent>('start');
  const [align, setAlign] = useState<AlignItems>('start');

  const cycleJustify = () => {
    setJustify((current) => JUSTIFY_VALUES[(JUSTIFY_VALUES.indexOf(current) + 1) % JUSTIFY_VALUES.length]);
  };
  const cycleAlign = () => {
    setAlign((current) => ALIGN_VALUES[(ALIGN_VALUES.indexOf(current) + 1) % ALIGN_VALUES.length]);
  };

  return (
    <>
      <hbox x={70} y={190} gap={18}>
        <Button
          sprite={{ src: BUTTON_FILES, mode: 'nineslice', bounds: BUTTON_BOUNDS, targetWidth: 360, targetHeight: 58 }}
          text={`justifyContent: ${justify}`}
          textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
          onPress={cycleJustify}
        />
        <Button
          sprite={{ src: BUTTON_FILES, mode: 'nineslice', bounds: BUTTON_BOUNDS, targetWidth: 300, targetHeight: 58 }}
          text={`alignItems: ${align}`}
          textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
          onPress={cycleAlign}
        />
        <text text="点击按钮循环切换；两块区域应同步变化。" fontSize={22} fillColor="#b8c2dc" />
      </hbox>

      <TestPanel x={70} y={300} width={780} height={650} title="4. Fixed VBox：520 × 500">
        <sprite
          src={PANEL_SRC}
          mode="nineslice"
          bounds={PANEL_BOUNDS}
          targetWidth={520}
          targetHeight={500}
          x={28}
          y={78}
          tint="#111827"
        />
        <vbox x={28} y={78} width={520} height={500} padding={20} gap={18} justifyContent={justify} alignItems={align}>
          <TestItem text="260 × 54" width={260} height={54} tint="#4f75c9" />
          <TestItem text="390 × 72" width={390} height={72} tint="#5f9b72" />
          <TestItem text="180 × 62" width={180} height={62} tint="#a66b75" />
        </vbox>
      </TestPanel>

      <TestPanel x={910} y={300} width={940} height={650} title="5. Fixed HBox：800 × 360">
        <sprite
          src={PANEL_SRC}
          mode="nineslice"
          bounds={PANEL_BOUNDS}
          targetWidth={800}
          targetHeight={360}
          x={28}
          y={78}
          tint="#111827"
        />
        <hbox x={28} y={78} width={800} height={360} padding={20} gap={20} justifyContent={justify} alignItems={align}>
          <TestItem text="150 × 100" width={150} height={100} tint="#4f75c9" />
          <TestItem text="190 × 180" width={190} height={180} tint="#5f9b72" />
          <TestItem text="130 × 72" width={130} height={72} tint="#a66b75" />
        </hbox>
        <text text="space-between：间隔应相等且不小于 gap=20。" fontSize={20} fillColor="#b8c2dc" x={28} y={470} />
      </TestPanel>
    </>
  );
}

function TransformTests() {
  const [dynamicMode, setDynamicMode] = useState(0);
  const dynamicModeLabels = ['初始布局', 'B 变宽', 'B 隐藏但占位', '删除 B 并重排'];

  return (
    <>
      <TestPanel
        x={70}
        y={190}
        width={1780}
        height={350}
        title="6. Direct child：x/y 是视觉偏移，不改变占位；anchor 被布局忽略"
      >
        <hbox x={28} y={95} gap={24} alignItems="center">
          <TestItem text="基准 A" width={190} height={70} tint="#4f75c9" />
          <Button
            sprite={{
              src: BUTTON_FILES,
              mode: 'nineslice',
              bounds: BUTTON_BOUNDS,
              targetWidth: 190,
              targetHeight: 70,
              tint: '#5f9b72',
            }}
            text="x=45"
            textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
            x={45}
          />
          <TestItem text="基准 B" width={190} height={70} tint="#a66b75" />
          <Button
            sprite={{
              src: BUTTON_FILES,
              mode: 'nineslice',
              bounds: BUTTON_BOUNDS,
              targetWidth: 190,
              targetHeight: 70,
              tint: '#b18448',
            }}
            text="anchor=.5"
            textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
            anchor={[0.5, 0.5]}
          />
          <TestItem text="基准 C" width={190} height={70} tint="#7d68b5" />
        </hbox>
        <text
          text="预期：x=45 的项目向右覆盖部分间隔，但 B 仍按原占位排列；anchor 项不应跳到父级中心。"
          fontSize={20}
          fillColor="#b8c2dc"
          x={28}
          y={270}
        />
      </TestPanel>

      <TestPanel x={70} y={590} width={1080} height={370} title="7. Transform 与普通 Container pivot 测量">
        <hbox x={28} y={100} gap={36} alignItems="center">
          <TestItem text="正常" width={190} height={80} tint="#4f75c9" />
          <container scale={0.7} rotation={-0.12}>
            <TestItem text="scale + rotation" width={240} height={100} tint="#5f9b72" />
          </container>
          <container>
            <sprite
              src={PANEL_SRC}
              mode="nineslice"
              bounds={PANEL_BOUNDS}
              targetWidth={240}
              targetHeight={80}
              tint="#7d68b5"
            />
            <text text="right aligned" fontSize={22} fillColor="#ffffff" x={220} y={25} pivot={[1, 0]} />
          </container>
          <TestItem text="后续项目" width={190} height={80} tint="#a66b75" />
        </hbox>
        <text
          text="预期：变换项按未变换尺寸占位；紫色项的右对齐文字不应扩大 240 宽度。"
          fontSize={20}
          fillColor="#b8c2dc"
          x={28}
          y={295}
        />
      </TestPanel>

      <TestPanel x={1200} y={590} width={650} height={370} title="8. 动态尺寸 / 隐藏 / 删除 / 重排">
        <Button
          sprite={{ src: BUTTON_FILES, mode: 'nineslice', bounds: BUTTON_BOUNDS, targetWidth: 250, targetHeight: 52 }}
          text={`下一状态：${dynamicModeLabels[(dynamicMode + 1) % dynamicModeLabels.length]}`}
          textStyle={{ fontSize: 21, glyphGridSize: 21, fillColor: '#ffffff' }}
          x={28}
          y={82}
          onPress={() => setDynamicMode((value) => (value + 1) % dynamicModeLabels.length)}
        />
        <hbox x={28} y={175} gap={18}>
          {dynamicMode === 3 ? <TestItem text="C" width={150} height={70} tint="#a66b75" /> : null}
          <TestItem text="A" width={150} height={70} tint="#4f75c9" />
          {dynamicMode !== 3 ? (
            <container visible={dynamicMode !== 2}>
              <TestItem text="B" width={dynamicMode === 0 ? 150 : 260} height={70} tint="#5f9b72" />
            </container>
          ) : null}
          {dynamicMode !== 3 ? <TestItem text="C" width={150} height={70} tint="#a66b75" /> : null}
        </hbox>
        <text
          text={`当前：${dynamicModeLabels[dynamicMode]}。每次点击应同帧完成，不应逐帧跳动。`}
          fontSize={20}
          fillColor="#b8c2dc"
          x={28}
          y={300}
        />
      </TestPanel>
    </>
  );
}

function CompatibilityTests() {
  const navigation = useNavigation();

  return (
    <TestPanel x={70} y={190} width={1780} height={770} title="9. Anchor / Pivot 与现有页面回归">
      <text
        text="逐个打开后检查居中、内部按钮位置和关闭交互。关闭 overlay 后会回到本测试页。"
        fontSize={24}
        fillColor="#d9e2f5"
        x={30}
        y={82}
      />
      <vbox x={30} y={150} gap={22}>
        <hbox gap={22}>
          <TestItem text="左侧基准" width={220} height={64} tint="#4f75c9" />
          <Button
            sprite={{ src: BUTTON_FILES, mode: 'nineslice', bounds: BUTTON_BOUNDS, targetWidth: 300, targetHeight: 64 }}
            text="Dialog 居中检查"
            textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
            onPress={() => uiActions.confirm('Dialog 应位于舞台正中央，内容与两个按钮应相对背景正确居中。')}
          />
          <Button
            sprite={{ src: BUTTON_FILES, mode: 'nineslice', bounds: BUTTON_BOUNDS, targetWidth: 300, targetHeight: 64 }}
            text="发送两条 Notification"
            textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
            onPress={() => {
              uiActions.notify('第一条通知：应在顶部水平居中', { duration: 3000 });
              uiActions.notify('第二条通知：应与第一条垂直排列', { duration: 3000 });
            }}
          />
        </hbox>
        <hbox gap={22}>
          <Button
            sprite={{ src: BUTTON_FILES, mode: 'nineslice', bounds: BUTTON_BOUNDS, targetWidth: 300, targetHeight: 64 }}
            text="打开 Settings"
            textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
            onPress={() => navigation.pushOverlay('settings')}
          />
          <Button
            sprite={{ src: BUTTON_FILES, mode: 'nineslice', bounds: BUTTON_BOUNDS, targetWidth: 300, targetHeight: 64 }}
            text="打开 Backlog 空状态"
            textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
            onPress={() => navigation.pushOverlay('backlog')}
          />
          <Button
            sprite={{ src: BUTTON_FILES, mode: 'nineslice', bounds: BUTTON_BOUNDS, targetWidth: 300, targetHeight: 64 }}
            text="打开 Menu"
            textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
            onPress={() => navigation.pushOverlay('menu')}
          />
        </hbox>
        <hbox gap={22}>
          <Button
            sprite={{ src: BUTTON_FILES, mode: 'nineslice', bounds: BUTTON_BOUNDS, targetWidth: 300, targetHeight: 64 }}
            text="转到 Title"
            textStyle={{ fontSize: 22, glyphGridSize: 22, fillColor: '#ffffff' }}
            onPress={() => navigation.navigate('title')}
          />
          <text text="Title 的四个按钮应以 x=960 为中心，不应整体偏向右侧。" fontSize={22} fillColor="#b8c2dc" />
        </hbox>
      </vbox>
      <text
        text="通过标准：没有整体半屏偏移；内部文字和按钮仍相对各自背景正确对齐。"
        fontSize={23}
        fillColor="#f7d98b"
        x={30}
        y={660}
      />
    </TestPanel>
  );
}

export function LayoutTest() {
  const stageSize = getStageSize();
  const scale = Math.min(stageSize.width / 1920, stageSize.height / 1080);
  const [page, setPage] = useState<TestPage>('basic');

  return (
    <container label="VBox HBox 手工测试页" x={stageSize.width / 2} y={stageSize.height / 2} scale={scale}>
      <sprite src="ui/mask.png" pivot={[0.5, 0.5]} />
      <container x={-960} y={-540}>
        <text text="VBox / HBox Layout Manual Test" fontSize={38} fillColor="#ffffff" x={70} y={36} />
        <text text="黄色标题是测试项；浅蓝文字是预期结果。" fontSize={21} fillColor="#b8c2dc" x={72} y={88} />
        <hbox x={850} y={38} gap={12}>
          {TEST_PAGES.map((item) => (
            <Button
              key={item.key}
              sprite={{
                src: BUTTON_FILES,
                mode: 'nineslice',
                bounds: BUTTON_BOUNDS,
                targetWidth: 225,
                targetHeight: 54,
                tint: page === item.key ? '#b18448' : '#4b5875',
              }}
              text={item.label}
              textStyle={{ fontSize: 21, glyphGridSize: 21, fillColor: '#ffffff' }}
              onPress={() => setPage(item.key)}
            />
          ))}
        </hbox>
        {page === 'basic' ? <BasicTests /> : null}
        {page === 'alignment' ? <AlignmentTests /> : null}
        {page === 'transform' ? <TransformTests /> : null}
        {page === 'compatibility' ? <CompatibilityTests /> : null}
      </container>
    </container>
  );
}
