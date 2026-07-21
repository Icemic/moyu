import { Button, Checkbox, ScrollView, Select, Slider, useScrollView } from '@momoyu-ink/kit';
import { useState } from 'react';
import { DemoChip, Panel } from '../components/chrome';
import {
  BUTTON_SPRITE,
  BUTTON_TEXT_STYLE,
  CHECKBOX_CHECKED_SPRITE,
  CHECKBOX_UNCHECKED_SPRITE,
  COLOR,
  ITEM_COLORS,
  SELECT_LIST,
  SELECT_OPTION,
  SELECT_TRIGGER,
  SLIDER_THUMB,
  SLIDER_TRACK,
  TEXT,
} from '../theme';

const GROUP_LABEL = { fontSize: 18, fillColor: COLOR.dim } as const;

function ButtonPanel() {
  const [pressCount, setPressCount] = useState(0);

  return (
    <Panel title="Button 按钮" width={600} height={460} note="点击计数、禁用态与文本对齐方式。">
      <vbox gap={16}>
        <text {...GROUP_LABEL} text="常态按钮" />
        <Button
          sprite={{ ...BUTTON_SPRITE, targetWidth: 400, targetHeight: 56 }}
          text={`已点击 ${pressCount} 次`}
          textStyle={BUTTON_TEXT_STYLE}
          onPress={() => setPressCount((count) => count + 1)}
        />
        <text {...GROUP_LABEL} text="禁用按钮" />
        <Button
          disabled
          opacity={0.55}
          sprite={{ ...BUTTON_SPRITE, targetWidth: 400, targetHeight: 56 }}
          text="禁用状态"
          textStyle={{ ...BUTTON_TEXT_STYLE, fillColor: COLOR.caption }}
        />
        <text {...GROUP_LABEL} text="锁定悬停与左右对齐" />
        <hbox gap={16}>
          <Button
            sprite={{ ...BUTTON_SPRITE, targetWidth: 272, targetHeight: 56 }}
            text="左对齐"
            textStyle={{ ...BUTTON_TEXT_STYLE, fontSize: 20, glyphGridSize: 20 }}
            lockOn="hover"
            textAlign="left"
            textOffsetX={14}
          />
          <Button
            sprite={{ ...BUTTON_SPRITE, targetWidth: 272, targetHeight: 56 }}
            text="右对齐"
            textStyle={{ ...BUTTON_TEXT_STYLE, fontSize: 20, glyphGridSize: 20 }}
            textAlign="right"
            textOffsetX={258}
          />
        </hbox>
      </vbox>
    </Panel>
  );
}

function CheckboxPanel() {
  const [checked, setChecked] = useState(true);

  return (
    <Panel title="Checkbox 勾选框" width={420} height={460} note="受控与非受控两种用法。">
      <vbox gap={24}>
        <text {...GROUP_LABEL} text="受控组件" />
        <hbox gap={18} alignItems="center">
          <Checkbox
            checked={checked}
            onCheckedChange={setChecked}
            uncheckedSprite={CHECKBOX_UNCHECKED_SPRITE}
            checkedSprite={CHECKBOX_CHECKED_SPRITE}
          />
          <text {...TEXT.body} text={checked ? '当前：已勾选' : '当前：未勾选'} />
        </hbox>
        <text {...GROUP_LABEL} text="非受控组件" />
        <hbox gap={18} alignItems="center">
          <Checkbox
            defaultChecked={false}
            uncheckedSprite={CHECKBOX_UNCHECKED_SPRITE}
            checkedSprite={CHECKBOX_CHECKED_SPRITE}
          />
          <text {...TEXT.body} text="内部维护状态" />
        </hbox>
      </vbox>
    </Panel>
  );
}

function SliderPanel() {
  const [sliderValue, setSliderValue] = useState(0.62);

  return (
    <Panel title="Slider 滑块" width={420} height={460} note="轨道宽度可随面板收窄。">
      <vbox gap={24}>
        <text {...GROUP_LABEL} text="受控滑块" />
        <Slider
          value={sliderValue}
          onValueChange={setSliderValue}
          track={{ ...SLIDER_TRACK, targetWidth: 300 }}
          thumb={SLIDER_THUMB}
        />
        <text {...TEXT.body} text={`当前值 ${sliderValue.toFixed(2)}`} />
        <text {...GROUP_LABEL} text="非受控滑块" />
        <Slider defaultValue={0.35} track={{ ...SLIDER_TRACK, targetWidth: 300 }} thumb={SLIDER_THUMB} />
        <text {...TEXT.caption} text="默认值 0.35，内部维护状态" />
      </vbox>
    </Panel>
  );
}

function SelectPanel() {
  const [selected, setSelected] = useState('spring');

  return (
    <Panel title="Select 下拉选择" width={728} height={420} note="下拉列表展开时会覆盖下方内容（zIndex）。">
      <container>
        <vbox zIndex={2} gap={16}>
          <text {...GROUP_LABEL} text="动画 / Shader 方案" />
          <Select
            value={selected}
            onValueChange={setSelected}
            options={[
              { text: '弹簧动画 Spring', value: 'spring' },
              { text: 'Shader 过渡 Transition', value: 'shader' },
              { text: 'Raw WGSL 着色器', value: 'raw' },
            ]}
            trigger={SELECT_TRIGGER}
            list={SELECT_LIST}
            option={SELECT_OPTION}
            textStyle={BUTTON_TEXT_STYLE}
          />
        </vbox>
        <text {...TEXT.body} text={`当前选中：${selected}`} y={140} />
        <text
          {...TEXT.caption}
          text="选项取自动画与 Shader 渲染管线；展开的列表会浮在下方的状态文字之上。"
          y={184}
          boxWidth={640}
          lineHeight={30}
        />
      </container>
    </Panel>
  );
}

function ScrollViewPanel() {
  const scroll = useScrollView({ viewportHeight: 290 });

  return (
    <Panel title="ScrollView 滚动视图" width={744} height={420} note="滚轮或拖拽滚动。">
      <ScrollView width={700} height={290} controller={scroll} clipProps={{ x: 20, y: 10 }} contentProps={{ gap: 12 }}>
        {Array.from({ length: 12 }, (_, index) => (
          <DemoChip
            // biome-ignore lint/suspicious/noArrayIndexKey: not a problem
            key={index}
            label={`列表项 ${String(index + 1).padStart(2, '0')}`}
            width={660}
            height={44}
            color={ITEM_COLORS[index % ITEM_COLORS.length]}
          />
        ))}
      </ScrollView>
    </Panel>
  );
}

export function ControlsPage() {
  return (
    <container>
      <vbox gap={24}>
        <hbox gap={32}>
          <ButtonPanel />
          <CheckboxPanel />
          <SliderPanel />
        </hbox>
        <hbox gap={32}>
          <SelectPanel />
          <ScrollViewPanel />
        </hbox>
      </vbox>
    </container>
  );
}
