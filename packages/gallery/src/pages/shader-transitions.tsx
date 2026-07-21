import { Button, Select, Slider, type MoyuShaderAttributes, type Node } from '@momoyu-ink/kit';
import { useRef, useState } from 'react';
import { Panel } from '../components/chrome';
import {
  BUTTON_SPRITE,
  BUTTON_TEXT_STYLE,
  COLOR,
  ITEM_COLORS,
  SELECT_LIST,
  SELECT_OPTION,
  SELECT_TRIGGER,
  SLIDER_THUMB,
  SLIDER_TRACK,
  TEXT,
} from '../theme';

type ShaderSource = NonNullable<MoyuShaderAttributes['shader']>;

const EFFECTS: Array<{ title: string; shader: ShaderSource }> = [
  { title: 'Crossfade', shader: { type: 'builtin', name: 'crossfade' } },
  { title: 'Wipe', shader: { type: 'builtin', name: 'wipe', direction: 'left', softness: 0.08 } },
  { title: 'Fade', shader: { type: 'builtin', name: 'fade', out: 0.4, hold: 0.2, in: 0.4, color: '#111827' } },
  { title: 'Push', shader: { type: 'builtin', name: 'push', direction: 'left' } },
  { title: 'Slideaway', shader: { type: 'builtin', name: 'slideaway', direction: 'right' } },
  { title: 'Zoom', shader: { type: 'builtin', name: 'zoom', startScale: 0.2, endScale: 1, origin: [0.5, 0.5] } },
  { title: 'Pixellate', shader: { type: 'builtin', name: 'pixellate', steps: 5 } },
  {
    title: 'Mask',
    shader: {
      type: 'builtin',
      name: 'mask',
      rule: 'generated/mask-rule-horizontal.png',
      softness: 0.08,
      reverse: false,
    },
  },
];

// Chinese labels for the internal state machine states.
const STATUS_LABELS: Record<string, string> = {
  stable: '就绪',
  preparing: '准备中',
  running: '播放中',
  finished: '已完成',
};

export function ShaderTransitionsPage() {
  const shaderRef = useRef<Node>(null);
  const [effectTitle, setEffectTitle] = useState(EFFECTS[0].title);
  const [durationValue, setDurationValue] = useState(0.5);
  const [running, setRunning] = useState(false);
  const [status, setStatus] = useState('stable');
  const effect = EFFECTS.find((item) => item.title === effectTitle) ?? EFFECTS[0];
  const duration = Math.round(250 + durationValue * 1750);

  const prepare = () => {
    if (running) {
      return;
    }

    setRunning(true);
    setStatus('preparing');
    shaderRef.current?.executeCommand({
      subCommand: 'prepare',
      fromChannel: 0,
      toChannel: 1,
      mode: 'static',
    });
  };

  return (
    <container>
      <Panel title="控制台" width={1504} height={140} x={0} y={0} zIndex={2}>
        <hbox x={20} y={56} gap={24} alignItems="center">
          <Select
            value={effect.title}
            onValueChange={setEffectTitle}
            disabled={running}
            options={EFFECTS.map((item) => ({ text: item.title, value: item.title }))}
            trigger={SELECT_TRIGGER}
            list={SELECT_LIST}
            option={SELECT_OPTION}
            textStyle={BUTTON_TEXT_STYLE}
            zIndex={2}
          />
          <Slider
            value={durationValue}
            onValueChange={setDurationValue}
            disabled={running}
            track={{ ...SLIDER_TRACK, targetWidth: 300 }}
            thumb={SLIDER_THUMB}
          />
          <text {...TEXT.body} text={`时长 ${duration} ms`} fillColor={COLOR.accent} />
          <Button
            sprite={{ ...BUTTON_SPRITE, targetWidth: 240 }}
            text={running ? '播放中' : '播放转场'}
            textStyle={BUTTON_TEXT_STYLE}
            disabled={running}
            onPress={prepare}
          />
          <text {...TEXT.body} text={`状态：${STATUS_LABELS[status] ?? status}`} fillColor={COLOR.accent} />
        </hbox>
      </Panel>

      <Panel
        title="转场画面"
        width={1504}
        height={740}
        x={0}
        y={172}
        note="按钮只发送 prepare；perform 由 onPrepared 触发，onFinished 解锁下一次播放。"
      >
        <shader
          ref={shaderRef}
          x={202}
          y={60}
          width={1100}
          height={620}
          shader={effect.shader}
          timeControl="transition"
          displayChannel={1}
          onPrepared={() => {
            setStatus('running');
            shaderRef.current?.executeCommand({ subCommand: 'perform', duration });
          }}
          onFinished={() => {
            setStatus('finished');
            setRunning(false);
          }}
        >
          <shader-slot channel={0} static>
            <container>
              <text text="FROM" fontSize={150} fillColor={ITEM_COLORS[0]} x={120} y={120} />
              <text text="通道 0 · 由 prepare 捕获" fontSize={30} fillColor={COLOR.text} x={130} y={320} />
              <text text="Moyu Shader Transition" fontSize={42} fillColor={ITEM_COLORS[1]} x={520} y={450} rotation={-0.08} />
            </container>
          </shader-slot>
          <shader-slot channel={1} static>
            <container>
              <text text="TO" fontSize={180} fillColor={ITEM_COLORS[2]} x={650} y={100} />
              <text text="通道 1 · 稳定显示" fontSize={30} fillColor={COLOR.accent} x={610} y={330} />
              <text text="GPU TRANSITION" fontSize={52} fillColor={ITEM_COLORS[3]} x={100} y={470} rotation={0.06} />
            </container>
          </shader-slot>
          <shader-slot channel={2} static space="shader">
            <sprite src="generated/mask-rule-horizontal.png" />
          </shader-slot>
        </shader>
      </Panel>
    </container>
  );
}
