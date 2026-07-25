import { Button, Checkbox, Radio, RadioGroup, ScrollView, Select, Slider, useScrollView } from '@momoyu-ink/kit';
import { useLingui } from '@lingui/react/macro';
import { useState } from 'react';
import { DemoChip, Panel } from '../components/chrome';
import {
  BUTTON_SPRITE,
  BUTTON_TEXT_STYLE,
  CHECKBOX_CHECKED_SPRITE,
  CHECKBOX_UNCHECKED_SPRITE,
  COLOR,
  ITEM_COLORS,
  RADIO_CHECKED_SPRITE,
  RADIO_UNCHECKED_SPRITE,
  SELECT_LIST,
  SELECT_OPTION,
  SELECT_TRIGGER,
  SLIDER_THUMB,
  SLIDER_TRACK,
  TEXT,
} from '../theme';

const GROUP_LABEL = { fontSize: 18, fillColor: COLOR.dim } as const;

function ButtonPanel() {
  const { t } = useLingui();
  const [pressCount, setPressCount] = useState(0);

  return (
    <Panel title={t`Button 按钮`} width={600} height={460} note={t`点击计数、禁用态与文本对齐方式。`}>
      <vbox gap={16}>
        <text {...GROUP_LABEL} text={t`常态按钮`} />
        <Button
          sprite={{ ...BUTTON_SPRITE, targetWidth: 400, targetHeight: 56 }}
          text={t`已点击 ${pressCount} 次`}
          textStyle={BUTTON_TEXT_STYLE}
          onPress={() => setPressCount((count) => count + 1)}
        />
        <text {...GROUP_LABEL} text={t`禁用按钮`} />
        <Button
          disabled
          opacity={0.55}
          sprite={{ ...BUTTON_SPRITE, targetWidth: 400, targetHeight: 56 }}
          text={t`禁用状态`}
          textStyle={{ ...BUTTON_TEXT_STYLE, fillColor: COLOR.caption }}
        />
        <text {...GROUP_LABEL} text={t`锁定悬停与左右对齐`} />
        <hbox gap={16}>
          <Button
            sprite={{ ...BUTTON_SPRITE, targetWidth: 272, targetHeight: 56 }}
            text={t`左对齐`}
            textStyle={{ ...BUTTON_TEXT_STYLE, fontSize: 20, glyphGridSize: 20 }}
            lockOn="hover"
            textAlign="left"
            textOffsetX={14}
          />
          <Button
            sprite={{ ...BUTTON_SPRITE, targetWidth: 272, targetHeight: 56 }}
            text={t`右对齐`}
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
  const { t } = useLingui();
  const [checked, setChecked] = useState(true);

  return (
    <Panel title={t`Checkbox 勾选框`} width={420} height={460} note={t`受控与非受控两种用法。`}>
      <vbox gap={24}>
        <text {...GROUP_LABEL} text={t`受控组件`} />
        <hbox gap={18} alignItems="center">
          <Checkbox
            checked={checked}
            onCheckedChange={setChecked}
            uncheckedSprite={CHECKBOX_UNCHECKED_SPRITE}
            checkedSprite={CHECKBOX_CHECKED_SPRITE}
          />
          <text {...TEXT.body} text={checked ? t`当前：已勾选` : t`当前：未勾选`} />
        </hbox>
        <text {...GROUP_LABEL} text={t`非受控组件`} />
        <hbox gap={18} alignItems="center">
          <Checkbox
            defaultChecked={false}
            uncheckedSprite={CHECKBOX_UNCHECKED_SPRITE}
            checkedSprite={CHECKBOX_CHECKED_SPRITE}
          />
          <text {...TEXT.body} text={t`内部维护状态`} />
        </hbox>
      </vbox>
    </Panel>
  );
}

function SliderPanel() {
  const { t } = useLingui();
  const [sliderValue, setSliderValue] = useState(0.62);

  return (
    <Panel title={t`Slider 滑块`} width={420} height={460} note={t`轨道宽度可随面板收窄。`}>
      <vbox gap={24}>
        <text {...GROUP_LABEL} text={t`受控滑块`} />
        <Slider
          value={sliderValue}
          onValueChange={setSliderValue}
          track={{ ...SLIDER_TRACK, targetWidth: 300 }}
          thumb={SLIDER_THUMB}
        />
        <text {...TEXT.body} text={t`当前值 ${sliderValue.toFixed(2)}`} />
        <text {...GROUP_LABEL} text={t`非受控滑块`} />
        <Slider defaultValue={0.35} track={{ ...SLIDER_TRACK, targetWidth: 300 }} thumb={SLIDER_THUMB} />
        <text {...TEXT.caption} text={t`默认值 0.35，内部维护状态`} />
      </vbox>
    </Panel>
  );
}

function RadioPanel() {
  const { t } = useLingui();
  const [selected, setSelected] = useState('1920');

  return (
    <Panel title={t`Radio 单选框`} width={1504} height={320} note={t`RadioGroup 统一管理同组互斥选择。`}>
      <hbox gap={120}>
        <vbox gap={18}>
          <text {...GROUP_LABEL} text={t`受控组件`} />
          <RadioGroup value={selected} onValueChange={setSelected}>
            <hbox gap={32}>
              {[
                { value: '1920', label: '1920' },
                { value: '1280', label: '1280' },
                { value: '800', label: '800' },
              ].map((option) => (
                <hbox key={option.value} gap={12} alignItems="center">
                  <Radio
                    value={option.value}
                    uncheckedSprite={RADIO_UNCHECKED_SPRITE}
                    checkedSprite={RADIO_CHECKED_SPRITE}
                  />
                  <text {...TEXT.body} text={option.label} />
                </hbox>
              ))}
            </hbox>
          </RadioGroup>
          <text {...TEXT.caption} text={t`当前值：${selected}`} />
        </vbox>
        <vbox gap={18}>
          <text {...GROUP_LABEL} text={t`非受控组件`} />
          <RadioGroup defaultValue="windowed">
            <hbox gap={32}>
              <hbox gap={12} alignItems="center">
                <Radio value="windowed" uncheckedSprite={RADIO_UNCHECKED_SPRITE} checkedSprite={RADIO_CHECKED_SPRITE} />
                <text {...TEXT.body} text={t`窗口`} />
              </hbox>
              <hbox gap={12} alignItems="center">
                <Radio
                  value="fullscreen"
                  uncheckedSprite={RADIO_UNCHECKED_SPRITE}
                  checkedSprite={RADIO_CHECKED_SPRITE}
                />
                <text {...TEXT.body} text={t`全屏`} />
              </hbox>
            </hbox>
          </RadioGroup>
          <text {...TEXT.caption} text={t`内部维护当前值`} />
        </vbox>
      </hbox>
    </Panel>
  );
}

function SelectPanel() {
  const { t } = useLingui();
  const [selected, setSelected] = useState('spring');

  return (
    <Panel title={t`Select 下拉选择`} width={728} height={420} note={t`下拉列表展开时会覆盖下方内容（zIndex）。`}>
      <container>
        <vbox zIndex={2} gap={16}>
          <text {...GROUP_LABEL} text={t`动画 / Shader 方案`} />
          <Select
            value={selected}
            onValueChange={setSelected}
            options={[
              { text: t`弹簧动画 Spring`, value: 'spring' },
              { text: t`Shader 过渡 Transition`, value: 'shader' },
              { text: t`Raw WGSL 着色器`, value: 'raw' },
            ]}
            trigger={SELECT_TRIGGER}
            list={SELECT_LIST}
            option={SELECT_OPTION}
            textStyle={BUTTON_TEXT_STYLE}
          />
        </vbox>
        <text {...TEXT.body} text={t`当前选中：${selected}`} y={140} />
        <text
          {...TEXT.caption}
          text={t`选项取自动画与 Shader 渲染管线；展开的列表会浮在下方的状态文字之上。`}
          y={184}
          boxWidth={640}
          lineHeight={30}
        />
      </container>
    </Panel>
  );
}

function ScrollViewPanel() {
  const { t } = useLingui();
  const scroll = useScrollView({ viewportHeight: 290 });

  return (
    <Panel title={t`ScrollView 滚动视图`} width={744} height={420} note={t`滚轮或拖拽滚动。`}>
      <ScrollView width={700} height={290} controller={scroll} clipProps={{ x: 20, y: 10 }} contentProps={{ gap: 12 }}>
        {Array.from({ length: 12 }, (_, index) => (
          <DemoChip
            // biome-ignore lint/suspicious/noArrayIndexKey: not a problem
            key={index}
            label={t`列表项 ${String(index + 1).padStart(2, '0')}`}
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
        <RadioPanel />
      </vbox>
    </container>
  );
}
