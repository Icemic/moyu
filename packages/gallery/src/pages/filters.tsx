import type { MoyuFilterAttributes } from '@momoyu-ink/kit';
import { useLingui } from '@lingui/react/macro';
import type { ReactNode } from 'react';
import { Panel } from '../components/chrome';
import { COLOR } from '../theme';

type Filters = NonNullable<MoyuFilterAttributes['filters']>;

function SampleContent() {
  const { t } = useLingui();

  return (
    <vbox gap={8}>
      <sprite src="images/sample.png" targetWidth={248} targetHeight={165} />
      <text text={t`滤镜文字 Aa123`} fontSize={22} fillColor={COLOR.text} />
      <text text={t`第二行 Second line`} fontSize={18} fillColor={COLOR.caption} />
    </vbox>
  );
}

function FilterCell({ title, note, filters }: { title: string; note: string; filters?: Filters }) {
  // The original cell renders identical content without a <filter> wrapper.
  let content: ReactNode = <SampleContent />;
  if (filters !== undefined) {
    content = <filter filters={filters}>{content}</filter>;
  }
  return (
    <Panel title={title} width={288} height={440} note={note}>
      {content}
    </Panel>
  );
}

export function FiltersPage() {
  const { t } = useLingui();
  const samples: Array<{ title: string; note: string; filters?: Filters }> = [
    { title: t`原始对照 Original`, note: t`无滤镜，作为对照基准。` },
    { title: t`模糊 Blur`, note: 'radius: 5', filters: [{ type: 'blur', radius: 5 }] },
    { title: t`亮度 Brightness`, note: 'amount: 1.55', filters: [{ type: 'brightness', amount: 1.55 }] },
    { title: t`对比度 Contrast`, note: 'amount: 1.75', filters: [{ type: 'contrast', amount: 1.75 }] },
    { title: t`饱和度 Saturation`, note: 'amount: 0.15', filters: [{ type: 'saturation', amount: 0.15 }] },
    { title: t`色相旋转 Hue Rotate`, note: 'degrees: 140', filters: [{ type: 'hue-rotate', degrees: 140 }] },
    { title: t`灰度 Grayscale`, note: 'amount: 1', filters: [{ type: 'grayscale', amount: 1 }] },
    {
      title: t`褐调 Sepia`,
      note: 'sepia 0.8 + contrast 1.25',
      filters: [
        { type: 'sepia', amount: 0.8 },
        { type: 'contrast', amount: 1.25 },
      ],
    },
    { title: t`反色 Invert`, note: 'amount: 1', filters: [{ type: 'invert', amount: 1 }] },
  ];
  const rows = [samples.slice(0, 5), samples.slice(5, 10)];
  return (
    <container>
      <vbox gap={16}>
        {rows.map((row, rowIndex) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: not a problem
          <hbox key={rowIndex} gap={16}>
            {row.map((sample) => (
              <FilterCell key={sample.title} title={sample.title} note={sample.note} filters={sample.filters} />
            ))}
          </hbox>
        ))}
      </vbox>
    </container>
  );
}
