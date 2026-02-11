import { type SpringRef, type SpringValues, useSpring } from '../spring';

export function useFadeInOut(
  inTime: number,
  _keepTime: number,
  _outTime: number,
  pause = false,
  onFinished?: () => void,
): [SpringValues<{ opacity: number }>, SpringRef<{ opacity: number }>, () => void] {
  const [style, api] = useSpring(
    () => ({
      from: { opacity: 0 },
      to: { opacity: 1 },
      config: { duration: inTime },
      pause,
      onRest: (result) => {
        if (result.value.opacity === 1) {
          void api.start({ reverse: true, delay: 1500, config: { duration: 1000 } });
        } else {
          onFinished?.();
        }
      },
    }),
    [],
  );

  const skip = () => {
    api.stop();
    api.set({ opacity: 0 });
    onFinished?.();
  };

  return [style, api, skip];
}

export function useFadeIn(
  inTime: number,
  pause = false,
  onFinished?: () => void,
): [SpringValues<{ opacity: number }>, SpringRef<{ opacity: number }>, () => void] {
  const [style, api] = useSpring(
    () => ({
      from: { opacity: 0 },
      to: { opacity: 1 },
      config: { duration: inTime },
      pause,
      onRest: () => {
        onFinished?.();
      },
    }),
    [],
  );

  const skip = () => {
    api.stop();
    api.set({ opacity: 1 });
    onFinished?.();
  };

  return [style, api, skip];
}

export function useFadeOut(
  outTime: number,
  pause = false,
  onFinished?: () => void,
): [SpringValues<{ opacity: number }>, SpringRef<{ opacity: number }>, () => void] {
  const [style, api] = useSpring(
    () => ({
      from: { opacity: 1 },
      to: { opacity: 0 },
      config: { duration: outTime },
      pause,
      onRest: () => {
        onFinished?.();
      },
    }),
    [],
  );

  const skip = () => {
    api.stop();
    api.set({ opacity: 1 });
    onFinished?.();
  };

  return [style, api, skip];
}
