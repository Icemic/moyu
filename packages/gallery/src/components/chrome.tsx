import { Button } from '@momoyu-ink/kit';
import type { ReactNode } from 'react';
import { BUTTON_SPRITE, BUTTON_TEXT_STYLE, COLOR, PANEL_SPRITE, TEXT, chipSprite } from '../theme';

/**
 * Titled panel — the basic framing unit of every gallery page.
 *
 * Children are positioned by the caller inside the panel; content
 * conventionally starts at x=20, y=56 (below the title). `note` pins an
 * expectation/description caption to the bottom edge.
 */
export function Panel({
  title,
  note,
  width,
  height,
  x,
  y,
  zIndex,
  children,
}: {
  title?: string;
  note?: string;
  width: number;
  height: number;
  x?: number;
  y?: number;
  zIndex?: number;
  children?: ReactNode;
}) {
  return (
    <sprite {...PANEL_SPRITE} targetWidth={width} targetHeight={height} x={x} y={y} zIndex={zIndex}>
      <vbox x={20} y={14} gap={8}>
        {title === undefined ? null : <text {...TEXT.panelTitle} text={title} />}
        {note === undefined ? null : <text {...TEXT.caption} text={note} boxWidth={width - 40} lineHeight={2} />}
        <container>{children}</container>
      </vbox>
    </sprite>
  );
}

/**
 * Colored demo block with a centered label, used to make layout and
 * measurement visible. Colors come from the original layout-test palette.
 */
export function DemoChip({
  label,
  width,
  height,
  color,
  fontSize = 20,
}: {
  label: string;
  width: number;
  height: number;
  color: string;
  fontSize?: number;
}) {
  return (
    <sprite {...chipSprite(color)} targetWidth={width} targetHeight={height}>
      <text text={label} fontSize={fontSize} fillColor="#ffffff" anchor={[0.5, 0.5]} pivot={[0.5, 0.5]} />
    </sprite>
  );
}

/**
 * Segmented tab row for switching sub-sections inside a page.
 */
export function SectionTabs<T extends string>({
  value,
  onChange,
  options,
  tabWidth = 220,
  x,
  y,
  zIndex,
}: {
  value: T;
  onChange: (value: T) => void;
  options: ReadonlyArray<{ value: T; label: string }>;
  tabWidth?: number;
  x?: number;
  y?: number;
  zIndex?: number;
}) {
  return (
    <hbox x={x} y={y} zIndex={zIndex} gap={12}>
      {options.map((option) => {
        const active = option.value === value;
        return (
          <Button
            key={option.value}
            sprite={{
              ...BUTTON_SPRITE,
              targetWidth: tabWidth,
              targetHeight: 48,
              tint: active ? COLOR.controlTintActive : COLOR.panelTint,
            }}
            text={option.label}
            textStyle={{
              ...BUTTON_TEXT_STYLE,
              fontSize: 20,
              glyphGridSize: 20,
              fillColor: active ? COLOR.accent : COLOR.navText,
            }}
            onPress={() => onChange(option.value)}
          />
        );
      })}
    </hbox>
  );
}
