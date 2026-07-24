import { Button, type EditableChangeSource, type EditableState, Input } from '@momoyu-ink/kit';
import { useState } from 'react';
import { Panel } from '../components/chrome';
import { BUTTON_SPRITE, BUTTON_TEXT_STYLE, COLOR, INPUT_BACKGROUND, INPUT_CARET, TEXT } from '../theme';

const INPUT_TEXT_STYLE = {
  fontSize: 28,
  glyphGridSize: 28,
  fillColor: COLOR.text,
} as const;

const PLACEHOLDER_STYLE = {
  fillColor: COLOR.dim,
} as const;

function stateSummary(state: EditableState): string {
  return `value="${state.value}" · display="${state.value}${state.compositionText}" · composition="${state.compositionText}"`;
}

export function InputPage() {
  const [controlledValue, setControlledValue] = useState('受控初始值');
  const [controlledState, setControlledState] = useState<EditableState | null>(null);
  const [readOnly, setReadOnly] = useState(true);
  const [disabled, setDisabled] = useState(true);
  const [lastEvent, setLastEvent] = useState('等待输入事件');

  const recordEvent = (name: string, state: EditableState, source?: EditableChangeSource) => {
    setLastEvent(`${name}${source ? ` (${source})` : ''} · ${stateSummary(state)}`);
  };

  return (
    <vbox gap={24}>
      <Panel
        title="基础输入与 IME"
        width={1504}
        height={400}
        note="悬停和按住输入框应切换背景；聚焦后应显示 focused 背景。再验证焦点切换、末尾输入、Backspace 与中文输入法候选窗位置。"
      >
        <vbox gap={18}>
          <text {...TEXT.caption} text="非受控输入（autoFocus）" />
          <Input
            defaultValue="Moyu <纯文本> "
            placeholder="请输入内容"
            autoFocus
            width={700}
            height={60}
            paddingX={16}
            textStyle={INPUT_TEXT_STYLE}
            placeholderStyle={PLACEHOLDER_STYLE}
            background={INPUT_BACKGROUND}
            caret={INPUT_CARET}
            onFocus={(state) => recordEvent('focus', state)}
            onBlur={(state) => recordEvent('blur', state)}
            onInput={(state) => recordEvent('input', state)}
            onChange={(state, source) => recordEvent('change', state, source)}
            onCompositionStart={(state) => recordEvent('compositionStart', state)}
            onCompositionUpdate={(state) => recordEvent('compositionUpdate', state)}
            onCompositionEnd={(state) => recordEvent('compositionEnd', state)}
          />
          <text {...TEXT.caption} text="第二个输入框（用于焦点切换）" />
          <Input
            defaultValue="第二个输入框"
            width={700}
            height={60}
            paddingX={16}
            textStyle={INPUT_TEXT_STYLE}
            background={INPUT_BACKGROUND}
            caret={INPUT_CARET}
            onFocus={(state) => recordEvent('focus', state)}
            onBlur={(state) => recordEvent('blur', state)}
            onChange={(state, source) => recordEvent('change', state, source)}
          />
          <text {...TEXT.caption} text={lastEvent} parseMarkup={false} boxWidth={1420} />
        </vbox>
      </Panel>

      <Panel
        title="变换后的候选窗位置"
        width={1504}
        height={300}
        note="使用 Microsoft 拼音输入时，候选窗应跟随缩放、旋转后的文字末尾，并保持在 caret 附近。"
      >
        <container x={180} y={80} scale={1.2} rotation={-0.04}>
          <Input
            defaultValue="Transform IME "
            width={700}
            height={60}
            paddingX={16}
            textStyle={INPUT_TEXT_STYLE}
            background={INPUT_BACKGROUND}
            caret={INPUT_CARET}
          />
        </container>
      </Panel>

      <Panel
        title="受控、只读与禁用"
        width={1504}
        height={500}
        note="受控值应跟随 onChange 回写；只读可聚焦但不能编辑；点击禁用输入会清除当前焦点。"
      >
        <hbox gap={48}>
          <vbox gap={18}>
            <text {...TEXT.caption} text="受控输入" />
            <Input
              value={controlledValue}
              width={700}
              height={60}
              paddingX={16}
              textStyle={INPUT_TEXT_STYLE}
              background={INPUT_BACKGROUND}
              caret={INPUT_CARET}
              onInput={setControlledState}
              onChange={(state) => {
                setControlledValue(state.value);
                setControlledState(state);
              }}
            />
            <Button
              sprite={{ ...BUTTON_SPRITE, targetWidth: 320, targetHeight: 52 }}
              text="程序设置受控值"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => setControlledValue('由外部状态设置')}
            />
            <text
              {...TEXT.caption}
              text={controlledState ? stateSummary(controlledState) : `value="${controlledValue}"`}
              parseMarkup={false}
              boxWidth={700}
            />
          </vbox>
          <vbox gap={18}>
            <text {...TEXT.caption} text="readOnly：允许聚焦，不允许输入" />
            <Input
              defaultValue="只读内容"
              readOnly={readOnly}
              width={700}
              height={60}
              paddingX={16}
              textStyle={INPUT_TEXT_STYLE}
              background={INPUT_BACKGROUND}
              caret={INPUT_CARET}
            />
            <Button
              sprite={{ ...BUTTON_SPRITE, targetWidth: 320, targetHeight: 52 }}
              text={readOnly ? '切换为可编辑' : '切换为只读'}
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => setReadOnly((current) => !current)}
            />
            <text {...TEXT.caption} text="disabled：点击后不获得焦点" />
            <Input
              defaultValue="禁用内容"
              disabled={disabled}
              opacity={disabled ? 0.5 : 1}
              width={700}
              height={60}
              paddingX={16}
              textStyle={INPUT_TEXT_STYLE}
              background={INPUT_BACKGROUND}
              caret={INPUT_CARET}
            />
            <Button
              sprite={{ ...BUTTON_SPRITE, targetWidth: 320, targetHeight: 52 }}
              text={disabled ? '启用输入框' : '禁用输入框'}
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => setDisabled((current) => !current)}
            />
          </vbox>
        </hbox>
      </Panel>
    </vbox>
  );
}
