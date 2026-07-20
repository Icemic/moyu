import { useState } from 'react';
import { Button, type ButtonProps } from './button';
import type { ControlSpriteProps } from './control';

export interface CheckboxProps extends Omit<ButtonProps, 'lockOn' | 'sprite'> {
  checked?: boolean;
  defaultChecked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  uncheckedSprite: ControlSpriteProps;
  checkedSprite: ControlSpriteProps;
}

export function Checkbox({
  checked,
  defaultChecked = false,
  onCheckedChange,
  uncheckedSprite,
  checkedSprite,
  onPress,
  ...buttonProps
}: CheckboxProps) {
  const [internalChecked, setInternalChecked] = useState(defaultChecked);
  const currentChecked = checked ?? internalChecked;

  return (
    <Button
      {...buttonProps}
      sprite={currentChecked ? checkedSprite : uncheckedSprite}
      onPress={(event) => {
        onPress?.(event);
        if (event.defaultPrevented) {
          return;
        }

        const nextChecked = !currentChecked;
        if (checked === undefined) {
          setInternalChecked(nextChecked);
        }
        onCheckedChange?.(nextChecked);
      }}
    />
  );
}
