import { useState } from 'react';
import type { SpriteProps } from '../bindings/SpriteProps';
import type { MoyuNodeAttributes } from '../declaration';
import { Button, type PressEvent } from './button';
import type { ControlSpriteProps, ControlStateValue, ControlTextStyle } from './control';

export interface SelectOption {
  text: string;
  value: string;
}

export interface SelectListProps extends Omit<SpriteProps, 'src' | 'targetHeight'> {
  src: string;
  paddingX?: number;
  paddingY?: number;
  gap?: number;
  offsetX?: number;
  offsetY?: number;
}

export interface SelectOptionSpriteProps extends Omit<ControlSpriteProps, 'targetHeight'> {
  targetHeight: number;
}

export interface SelectProps extends Omit<MoyuNodeAttributes, 'onClick'> {
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string, option: SelectOption) => void;
  onPress?: (event: PressEvent) => void;
  disabled?: boolean;
  options: readonly SelectOption[];
  trigger: ControlSpriteProps;
  list: SelectListProps;
  option: SelectOptionSpriteProps;
  textStyle?: ControlStateValue<ControlTextStyle>;
  textOffsetX?: number;
  textOffsetY?: number;
  textAlign?: 'left' | 'center' | 'right';
}

export function Select({
  value,
  defaultValue,
  onValueChange,
  onPress,
  disabled = false,
  options,
  trigger,
  list,
  option,
  textStyle,
  textOffsetX,
  textOffsetY,
  textAlign,
  anchor,
  pivot = anchor,
  ...containerProps
}: SelectProps) {
  const [internalValue, setInternalValue] = useState(defaultValue);
  const [open, setOpen] = useState(false);
  const currentValue = value ?? internalValue;
  const currentOption = options.find((item) => item.value === currentValue);
  const { paddingX = 0, paddingY = 0, gap = 0, offsetX = 0, offsetY = 0, ...listSprite } = list;
  const listHeight = paddingY * 2 + options.length * option.targetHeight + Math.max(0, options.length - 1) * gap;

  return (
    <container
      {...containerProps}
      anchor={anchor}
      pivot={pivot}
      interactive={disabled ? false : containerProps.interactive}
    >
      <Button
        sprite={trigger}
        disabled={disabled}
        lockOn={open ? 'press' : undefined}
        text={currentOption?.text ?? ''}
        textStyle={textStyle}
        textOffsetX={textOffsetX}
        textOffsetY={textOffsetY}
        textAlign={textAlign}
        onPress={(event) => {
          onPress?.(event);
          if (!event.defaultPrevented) {
            setOpen((active) => !active);
          }
        }}
      />
      {open && !disabled ? (
        <sprite {...listSprite} targetHeight={listHeight} x={offsetX} y={(trigger.targetHeight ?? 0) + offsetY}>
          <vbox x={paddingX} y={paddingY} gap={gap}>
            {options.map((item) => (
              <Button
                key={item.value}
                sprite={option}
                lockOn={item.value === currentValue ? 'press' : undefined}
                text={item.text}
                textStyle={textStyle}
                textOffsetX={textOffsetX}
                textOffsetY={textOffsetY}
                textAlign={textAlign}
                onPress={(event) => {
                  if (event.defaultPrevented) {
                    return;
                  }
                  if (value === undefined) {
                    setInternalValue(item.value);
                  }
                  onValueChange?.(item.value, item);
                  setOpen(false);
                }}
              />
            ))}
          </vbox>
        </sprite>
      ) : undefined}
    </container>
  );
}
