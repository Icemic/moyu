import { useEffect, useState } from 'react';
import { addEventListener, type MouseEvent } from '../events';

/**
 * Whether the last input came from a touch, so a component can adapt to a device whose
 * input has no hover — buttons that a mouse reveals on hover would otherwise stay hidden
 * on a touch device.
 *
 * The state is shared by every caller: the answer is a property of the device, not of one
 * component, and each instance listening on its own would only duplicate the work.
 */
let touchInput = false;
const subscribers = new Set<(value: boolean) => void>();
let uninstall: (() => void) | null = null;

function setTouchInput(value: boolean) {
  if (value === touchInput) {
    return;
  }

  touchInput = value;
  for (const notify of [...subscribers]) {
    notify(value);
  }
}

function install() {
  if (uninstall) {
    return;
  }

  const cleanups = [
    // A touch means the device is being used by hand.
    addEventListener('touchstart', () => setTouchInput(true)),
    // Only a mouse event the engine did not produce on its own means the user reached
    // for the mouse: a tap's mouse compatibility gesture and the hover refresh a frame
    // repeats both carry `synthetic`.
    addEventListener('mousemove', (event: MouseEvent) => {
      if (!event.synthetic) {
        setTouchInput(false);
      }
    }),
  ];

  uninstall = () => {
    for (const cleanup of cleanups) {
      cleanup();
    }
    uninstall = null;
  };
}

export function useTouchInput(): boolean {
  const [value, setValue] = useState(touchInput);

  useEffect(() => {
    subscribers.add(setValue);
    // Catch a change that happened between render and subscribe.
    setValue(touchInput);
    install();

    return () => {
      subscribers.delete(setValue);
      if (subscribers.size === 0) {
        uninstall?.();
      }
    };
  }, []);

  return value;
}
