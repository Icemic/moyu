import { useLingui } from '@lingui/react/macro';
import { Button, executePluginCommand, type MoyuNodeAttributes } from '@momoyu-ink/kit';
import { useEffect, useState } from 'react';
import { Panel } from '../components/chrome';
import { BUTTON_SPRITE, BUTTON_TEXT_STYLE, COLOR, ITEM_COLORS, chipSprite } from '../theme';

/** The `cursor` values a node accepts. */
type MoyuCursor = NonNullable<MoyuNodeAttributes['cursor']>;

// Panel content starts at x = 20 and is 1504 wide, leaving 1464 for the grid.
const CHIP_WIDTH = 354;
const CHIP_HEIGHT = 120;
const CHIP_GAP = 16;
const ROW_LENGTH = 4;

/**
 * The set this page enables on demand. The project itself configures no cursor, so
 * every other page keeps the system pointers.
 */
const DEMO_CURSORS = {
  default: { src: 'cursors/arrow.png', hotspot: [1, 1] as [number, number] },
  pointer: { src: 'cursors/hand.png', hotspot: [14, 3] as [number, number] },
  text: { src: 'cursors/ibeam.png', hotspot: [16, 16] as [number, number] },
  crosshair: { src: 'cursors/crosshair.png', hotspot: [16, 16] as [number, number] },
  move: { src: 'cursors/move.png', hotspot: [16, 16] as [number, number] },
};

/** Configurations the switcher can apply while running. */
const PRESET_CONFIGS = {
  none: {},
  demo: DEMO_CURSORS,
  all: 'cursors/crosshair.png',
  'pointer-only': { pointer: { src: 'cursors/hand.png', hotspot: [14, 3] as [number, number] } },
} as const;

type PresetValue = keyof typeof PRESET_CONFIGS;

/** Hand one configuration to the engine. */
function applyPreset(preset: PresetValue) {
  void executePluginCommand('system', {
    subCommand: 'setCustomCursors',
    cursors: PRESET_CONFIGS[preset],
  });
}

function CursorChip({
  cursor,
  label,
  note,
  color,
}: {
  cursor: MoyuCursor;
  label: string;
  note: string;
  color: string;
}) {
  return (
    <sprite {...chipSprite(color)} targetWidth={CHIP_WIDTH} targetHeight={CHIP_HEIGHT} cursor={cursor} interactive>
      <vbox x={18} y={16} gap={4}>
        <text text={label} fontSize={24} fillColor="#ffffff" />
        <text text={note} fontSize={18} fillColor="#dbe4f3" />
      </vbox>
    </sprite>
  );
}

/**
 * Shows inheritance and override in one chip: the outer container sets
 * `crosshair`, its unset child follows it, and a deeper container sets `pointer`.
 */
function NestedCursorChip({ color }: { color: string }) {
  const { t } = useLingui();

  return (
    <sprite {...chipSprite(color)} targetWidth={CHIP_WIDTH} targetHeight={CHIP_HEIGHT} interactive>
      <container x={18} y={16} cursor="crosshair">
        <vbox gap={4}>
          <text text={t`外层容器：crosshair`} fontSize={22} fillColor="#ffffff" />
          <container cursor="pointer">
            <text text={t`内层设为 pointer`} fontSize={18} fillColor="#dbe4f3" />
          </container>
        </vbox>
      </container>
    </sprite>
  );
}

export function CursorPage() {
  const { t } = useLingui();
  const [preset, setPreset] = useState<PresetValue>('none');

  // This page is the only place that configures pointers, so it hands them back
  // when it goes away; every other page keeps the system ones.
  useEffect(() => () => applyPreset('none'), []);

  const select = (next: PresetValue) => {
    setPreset(next);
    applyPreset(next);
  };

  const presets: ReadonlyArray<{ value: PresetValue; label: string }> = [
    { value: 'none', label: t`系统指针` },
    { value: 'demo', label: t`示例图片` },
    { value: 'all', label: t`一张图通吃` },
    { value: 'pointer-only', label: t`只配 pointer` },
  ];

  const states: ReadonlyArray<{ cursor: MoyuCursor; label: string; note: string }> = [
    { cursor: 'default', label: 'default', note: t`未配置状态的回退` },
    { cursor: 'pointer', label: 'pointer', note: t`可点击对象` },
    { cursor: 'text', label: 'text', note: t`文本选择` },
    { cursor: 'crosshair', label: 'crosshair', note: t`精确取点` },
    { cursor: 'move', label: 'move', note: t`可拖动对象` },
    { cursor: 'grab', label: 'grab', note: t`未配置，落到 default` },
    { cursor: 'wait', label: 'wait', note: t`未配置，落到 default` },
    { cursor: 'hidden', label: 'hidden', note: t`隐藏指针，不经过图片` },
  ];

  // The states fill whole rows; the leftover slots carry the nested case.
  const cells = Array.from({ length: states.length + 1 }, (_, index) => states[index] ?? null);
  const rows = Array.from({ length: Math.ceil(cells.length / ROW_LENGTH) }, (_, index) =>
    cells.slice(index * ROW_LENGTH, (index + 1) * ROW_LENGTH),
  );

  return (
    <vbox gap={24}>
      <Panel
        title={t`运行期切换`}
        width={1504}
        height={220}
        note={t`system 命令 setCustomCursors 接受与 index.json 相同的定义；传入空对象即清空，离开本页也会自动清空。下方色块展示当前配置下各状态的指针效果。`}
      >
        <hbox gap={CHIP_GAP} y={8}>
          {presets.map((item) => (
            <Button
              key={item.value}
              sprite={{
                ...BUTTON_SPRITE,
                targetWidth: 340,
                targetHeight: 56,
                tint: item.value === preset ? COLOR.controlTintActive : COLOR.panelTint,
              }}
              text={item.label}
              textStyle={{
                ...BUTTON_TEXT_STYLE,
                fontSize: 20,
                fillColor: item.value === preset ? COLOR.accent : COLOR.navText,
              }}
              onPress={() => select(item.value)}
            />
          ))}
        </hbox>
      </Panel>

      <Panel
        title={t`鼠标指针`}
        width={1504}
        height={560}
        note={t`节点的 cursor 属性按状态选用图片，未单独配置的状态使用 default；hotspot 是图片中响应指针的位置，按图片像素指定，箭头为 (1, 1)，手势 (14, 3)，其余均为 (16, 16)。最后一格演示子节点继承父容器的指针。`}
      >
        <vbox gap={CHIP_GAP}>
          {rows.map((row, rowIndex) => (
            <hbox key={rowIndex} gap={CHIP_GAP}>
              {row.map((state, index) => {
                const color = ITEM_COLORS[(rowIndex * ROW_LENGTH + index) % ITEM_COLORS.length];

                return state === null ? (
                  <NestedCursorChip key="nested" color={color} />
                ) : (
                  <CursorChip
                    key={state.cursor}
                    cursor={state.cursor}
                    label={state.label}
                    note={state.note}
                    color={color}
                  />
                );
              })}
            </hbox>
          ))}
        </vbox>
      </Panel>
    </vbox>
  );
}
