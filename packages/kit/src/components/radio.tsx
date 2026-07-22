import { createContext, useContext, useState, type ReactNode } from 'react';
import type { MoyuNodeAttributes } from '../declaration';
import { Button, type ButtonProps } from './button';
import type { ControlSpriteProps } from './control';

interface RadioGroupContextValue {
  value: string | undefined;
  select: (value: string) => void;
}

const RadioGroupContext = createContext<RadioGroupContextValue | null>(null);

export interface RadioGroupProps extends MoyuNodeAttributes {
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  children?: ReactNode;
}

export function RadioGroup({ value, defaultValue, onValueChange, children, ...containerProps }: RadioGroupProps) {
  const [internalValue, setInternalValue] = useState(defaultValue);
  const currentValue = value ?? internalValue;

  return (
    <RadioGroupContext.Provider
      value={{
        value: currentValue,
        select: (nextValue) => {
          if (nextValue === currentValue) {
            return;
          }
          if (value === undefined) {
            setInternalValue(nextValue);
          }
          onValueChange?.(nextValue);
        },
      }}
    >
      <container {...containerProps}>{children}</container>
    </RadioGroupContext.Provider>
  );
}

export interface RadioProps extends Omit<ButtonProps, 'lockOn' | 'sprite'> {
  value: string;
  uncheckedSprite: ControlSpriteProps;
  checkedSprite: ControlSpriteProps;
}

export function Radio({ value, uncheckedSprite, checkedSprite, onPress, ...buttonProps }: RadioProps) {
  const group = useContext(RadioGroupContext);
  if (!group) {
    throw new Error('Radio must be rendered inside a RadioGroup.');
  }
  const checked = group.value === value;

  return (
    <Button
      {...buttonProps}
      sprite={checked ? checkedSprite : uncheckedSprite}
      onPress={(event) => {
        onPress?.(event);
        if (!event.defaultPrevented) {
          group.select(value);
        }
      }}
    />
  );
}
