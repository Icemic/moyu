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

type RawShaderSource = Extract<NonNullable<MoyuShaderAttributes['shader']>, { type: 'raw' }>;
type RawTimeControl = Exclude<NonNullable<MoyuShaderAttributes['timeControl']>, 'transition'>;

const RAW_SHADER_HEADER = `
struct ParamsUniform {
  slots: array<vec4<u32>, 8>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@group(1) @binding(3)
var<uniform> params: ParamsUniform;

fn read_param_u32(index: u32) -> u32 {
  let lane = params.slots[index / 4u];
  switch (index % 4u) {
    case 0u: { return lane.x; }
    case 1u: { return lane.y; }
    case 2u: { return lane.z; }
    default: { return lane.w; }
  }
}

fn read_param_f32(index: u32) -> f32 {
  return bitcast<f32>(read_param_u32(index));
}
`;

const PRESETS: Array<{ title: string; description: string; content: string }> = [
  {
    title: 'Color shift',
    description: '随时间偏移红蓝通道采样。',
    content: `${RAW_SHADER_HEADER}
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let strength = read_param_f32(0u);
  let speed = read_param_f32(1u);
  let offset = sin(builtins.time * speed) * 0.02 * strength;
  let center = sampleChannel0(input.uv);
  let red = sampleChannel0(vec2<f32>(input.uv.x + offset, input.uv.y)).r;
  let blue = sampleChannel0(vec2<f32>(input.uv.x - offset, input.uv.y)).b;
  return vec4<f32>(red, center.g, blue, center.a);
}
`,
  },
  {
    title: 'Wave',
    description: '用移动的正弦波横向扭曲输入画面。',
    content: `${RAW_SHADER_HEADER}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let strength = read_param_f32(0u);
  let speed = read_param_f32(1u);
  let wave = sin((input.uv.y * 9.0 + builtins.time * speed) * 6.28318) * 0.015 * strength;
  let color = sampleChannel0(vec2<f32>(input.uv.x + wave, input.uv.y));
  let pulse = 0.75 + 0.25 * sin(builtins.time * speed * 3.0 + input.uv.x * 8.0);
  let shifted = vec3<f32>(color.r * pulse, color.g, color.b * (1.1 - pulse * 0.25));
  return vec4<f32>(mix(color.rgb, shifted, strength), color.a);
}
`,
  },
  {
    title: 'Scan',
    description: '一道明亮的扫描带扫过输入画面。',
    content: `${RAW_SHADER_HEADER}
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let strength = read_param_f32(0u);
  let speed = read_param_f32(1u);
  let color = sampleChannel0(input.uv);
  let scan_position = fract(builtins.time * speed * 0.2);
  let distance = abs(input.uv.y - scan_position);
  let band = smoothstep(0.08, 0.0, distance) * strength;
  let scan_color = color.rgb + vec3<f32>(0.15, 0.35, 0.5) * band;
  return vec4<f32>(scan_color, color.a);
}
`,
  },
];

export function RawShadersPage() {
  const shaderRef = useRef<Node>(null);
  const [presetTitle, setPresetTitle] = useState(PRESETS[0].title);
  const [timeControl, setTimeControl] = useState<RawTimeControl>('manual');
  const [playing, setPlaying] = useState(false);
  const [strength, setStrength] = useState(0.55);
  const preset = PRESETS.find((item) => item.title === presetTitle) ?? PRESETS[0];
  const shader: RawShaderSource = {
    type: 'raw',
    content: preset.content,
    params: [
      { name: 'strength', type: 'float', value: strength },
      { name: 'speed', type: 'float', value: 1.4 },
    ],
  };

  return (
    <container>
      <vbox gap={16}>
        <Panel title="控制台" width={1504} height={200} zIndex={2}>
          <vbox gap={16}>
            <hbox gap={16} alignItems="center" zIndex={2}>
              <Select
                value={preset.title}
                onValueChange={setPresetTitle}
                options={PRESETS.map((item) => ({ text: item.title, value: item.title }))}
                trigger={SELECT_TRIGGER}
                list={SELECT_LIST}
                option={SELECT_OPTION}
                textStyle={BUTTON_TEXT_STYLE}
              />
              <Select
                value={timeControl}
                onValueChange={(value) => {
                  shaderRef.current?.executeCommand({ subCommand: 'stop' });
                  setPlaying(false);
                  setTimeControl(value as RawTimeControl);
                }}
                options={[
                  { text: '手动时间', value: 'manual' },
                  { text: '自动时间', value: 'auto' },
                ]}
                trigger={{ ...SELECT_TRIGGER, targetWidth: 260 }}
                list={{ ...SELECT_LIST, targetWidth: 260 }}
                option={{ ...SELECT_OPTION, targetWidth: 254 }}
                textStyle={BUTTON_TEXT_STYLE}
              />
              <Slider value={strength} onValueChange={setStrength} track={SLIDER_TRACK} thumb={SLIDER_THUMB} />
              <text {...TEXT.body} text={`强度 ${strength.toFixed(2)}`} fillColor={COLOR.accent} />
            </hbox>

            <hbox gap={16} alignItems="center">
              <Button
                sprite={{ ...BUTTON_SPRITE, targetWidth: 180 }}
                text={playing ? '停止' : '启动'}
                textStyle={BUTTON_TEXT_STYLE}
                disabled={timeControl === 'auto'}
                opacity={timeControl === 'auto' ? 0.4 : 1}
                onPress={() => {
                  if (playing) {
                    shaderRef.current?.executeCommand({ subCommand: 'stop' });
                  } else {
                    shaderRef.current?.executeCommand({ subCommand: 'start' });
                  }
                  setPlaying((value) => !value);
                }}
              />
              <Button
                sprite={{ ...BUTTON_SPRITE, targetWidth: 180 }}
                text="重置"
                textStyle={BUTTON_TEXT_STYLE}
                disabled={timeControl === 'auto'}
                opacity={timeControl === 'auto' ? 0.4 : 1}
                onPress={() => {
                  shaderRef.current?.executeCommand({ subCommand: 'reset' });
                  setPlaying(false);
                }}
              />
              <text {...TEXT.caption} text={preset.description} />
            </hbox>
          </vbox>
        </Panel>

        <Panel title="渲染结果" width={1504} height={680}>
          <shader
            ref={shaderRef}
            x={202}
            y={30}
            width={1100}
            height={620}
            shader={shader}
            timeControl={timeControl}
            displayChannel={0}
          >
            <shader-slot channel={0}>
              <container>
                <text text="RAW WGSL" fontSize={130} fillColor={ITEM_COLORS[0]} x={180} y={100} />
                <text text="手动时间控制" fontSize={50} fillColor={ITEM_COLORS[2]} x={480} y={300} rotation={-0.06} />
                <text
                  text="参数槽 · 通道采样 · builtins.time"
                  fontSize={30}
                  fillColor={ITEM_COLORS[1]}
                  x={120}
                  y={470}
                />
              </container>
            </shader-slot>
          </shader>
        </Panel>
      </vbox>
    </container>
  );
}
