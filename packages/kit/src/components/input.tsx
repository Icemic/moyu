import { forwardRef, useCallback, useEffect, useImperativeHandle, useRef, useState } from 'react';
import type { EditableChangeSource } from '../bindings/EditableChangeSource';
import type { EditableEvent } from '../bindings/EditableEvent';
import type { EditableState } from '../bindings/EditableState';
import type { TextLayoutEvent } from '../bindings/TextLayoutEvent';
import type { MoyuNodeAttributes, MoyuSpriteAttributes, MoyuTextAttributes } from '../declaration';
import { addEventListener } from '../events';
import { blurEditable, focusEditable } from '../moyu';
import type { Node } from '../node';
import { mergeEvent } from '../utils';

export interface InputCaretStyle
  extends Omit<
    MoyuSpriteAttributes,
    | 'bounds'
    | 'children'
    | 'interactive'
    | 'mode'
    | 'nineSliceMode'
    | 'targetWidth'
    | 'targetHeight'
    | 'x'
    | 'y'
    | 'visible'
  > {
  width: number;
  height?: number;
  blinkInterval?: number;
}

export interface InputBackgroundStyle
  extends Omit<MoyuSpriteAttributes, 'children' | 'interactive' | 'targetWidth' | 'targetHeight'> {}

export interface InputBackgroundStates {
  idle: InputBackgroundStyle;
  hover?: InputBackgroundStyle;
  press?: InputBackgroundStyle;
  focused?: InputBackgroundStyle;
  readOnly?: InputBackgroundStyle;
  disabled?: InputBackgroundStyle;
}

export type InputBackground = InputBackgroundStyle | InputBackgroundStates;

export interface InputProps
  extends Omit<MoyuNodeAttributes, 'children' | 'onFocus' | 'onBlur' | 'onInput' | 'onChange'> {
  value?: string;
  defaultValue?: string;
  placeholder?: string;
  disabled?: boolean;
  readOnly?: boolean;
  autoFocus?: boolean;
  width: number;
  height: number;
  paddingX?: number;
  textStyle: Omit<MoyuTextAttributes, 'children' | 'interactive' | 'text'>;
  placeholderStyle?: Omit<MoyuTextAttributes, 'children' | 'interactive' | 'text'>;
  caret: InputCaretStyle;
  background?: InputBackground;
  onInput?: (state: EditableState) => void;
  onChange?: (state: EditableState, source: EditableChangeSource) => void;
  onFocus?: (state: EditableState) => void;
  onBlur?: (state: EditableState) => void;
  onCompositionStart?: (state: EditableState) => void;
  onCompositionUpdate?: (state: EditableState) => void;
  onCompositionEnd?: (state: EditableState) => void;
}

export interface InputHandle {
  focus(): void;
  blur(): void;
  getState(): EditableState;
}

export const Input = forwardRef<InputHandle, InputProps>(function Input(
  {
    value,
    defaultValue = '',
    placeholder = '',
    disabled = false,
    readOnly = false,
    autoFocus = false,
    width,
    height,
    paddingX = 0,
    textStyle,
    placeholderStyle,
    caret,
    background,
    onInput,
    onChange,
    onFocus,
    onBlur,
    onCompositionStart,
    onCompositionUpdate,
    onCompositionEnd,
    interactive,
    cursor = 'text',
    onMouseEnter,
    onMouseLeave,
    onMouseDown,
    onMouseUp,
    onTouchStart,
    onTouchEnd,
    onTouchCancel,
    ...nodeProps
  },
  ref,
) {
  const editableRef = useRef<Node>(null);
  const initialValueRef = useRef(value ?? defaultValue);
  const initialValue = initialValueRef.current;
  const stateRef = useRef<EditableState>({
    value: initialValue,
    isComposing: false,
    compositionText: '',
  });
  const [state, setState] = useState(stateRef.current);
  const [focused, setFocused] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [pressed, setPressed] = useState(false);
  const [caretPosition, setCaretPosition] = useState<[number, number]>();
  const [caretVisible, setCaretVisible] = useState(true);
  const shouldAutoFocusRef = useRef(autoFocus && !disabled);
  const viewportWidth = Math.max(0, Math.round(width));
  const viewportHeight = Math.max(0, Math.round(height));
  const contentWidth = Math.max(0, viewportWidth - paddingX * 2);
  const { width: caretWidth, height: configuredCaretHeight, blinkInterval, ...caretSpriteProps } = caret;
  const caretHeight = configuredCaretHeight ?? textStyle.fontSize ?? viewportHeight;
  const backgroundStyle =
    background && 'idle' in background
      ? disabled
        ? (background.disabled ?? background.idle)
        : readOnly
          ? (background.readOnly ?? background.idle)
          : pressed
            ? (background.press ?? background.idle)
            : focused
              ? (background.focused ?? background.idle)
              : hovered
                ? (background.hover ?? background.idle)
                : background.idle
      : background;
  const displayValue = state.value + state.compositionText;

  const updateState = useCallback((nextState: EditableState) => {
    const previousState = stateRef.current;
    if (
      previousState.value === nextState.value &&
      previousState.isComposing === nextState.isComposing &&
      previousState.compositionText === nextState.compositionText
    ) {
      return;
    }
    const displayValueChanged =
      previousState.value + previousState.compositionText !== nextState.value + nextState.compositionText;
    stateRef.current = nextState;
    setState(nextState);
    if (displayValueChanged) {
      setCaretPosition(undefined);
    }
    setCaretVisible(true);
  }, []);

  const handleInput = useCallback(
    (event: Extract<EditableEvent, { type: 'input' }>) => {
      updateState(event.state);
      onInput?.(event.state);
    },
    [onInput, updateState],
  );

  const handleChange = useCallback(
    (event: Extract<EditableEvent, { type: 'change' }>) => {
      updateState(event.state);
      onChange?.(event.state, event.source);
    },
    [onChange, updateState],
  );

  const handleFocus = useCallback(
    (event: Extract<EditableEvent, { type: 'focus' }>) => {
      updateState(event.state);
      setFocused(true);
      onFocus?.(event.state);
    },
    [onFocus, updateState],
  );

  const handleBlur = useCallback(
    (event: Extract<EditableEvent, { type: 'blur' }>) => {
      updateState(event.state);
      setFocused(false);
      onBlur?.(event.state);
    },
    [onBlur, updateState],
  );

  const handleCompositionStart = useCallback(
    (event: Extract<EditableEvent, { type: 'compositionStart' }>) => {
      updateState(event.state);
      onCompositionStart?.(event.state);
    },
    [onCompositionStart, updateState],
  );
  const handleCompositionUpdate = useCallback(
    (event: Extract<EditableEvent, { type: 'compositionUpdate' }>) => {
      updateState(event.state);
      onCompositionUpdate?.(event.state);
    },
    [onCompositionUpdate, updateState],
  );
  const handleCompositionEnd = useCallback(
    (event: Extract<EditableEvent, { type: 'compositionEnd' }>) => {
      updateState(event.state);
      onCompositionEnd?.(event.state);
    },
    [onCompositionEnd, updateState],
  );

  const handleTextLayout = useCallback((event: TextLayoutEvent) => {
    if (event.text === stateRef.current.value + stateRef.current.compositionText) {
      setCaretPosition(event.endCursorPosition);
    }
  }, []);

  useImperativeHandle(
    ref,
    () => ({
      focus: () => {
        if (editableRef.current) {
          focusEditable(editableRef.current.nodeId);
        }
      },
      blur: () => {
        if (editableRef.current) {
          blurEditable(editableRef.current.nodeId);
        }
      },
      getState: () => stateRef.current,
    }),
    [],
  );

  useEffect(() => {
    if (value === undefined || value === stateRef.current.value) {
      return;
    }
    editableRef.current?.executeCommand({ subCommand: 'setValue', value });
    const nextState = editableRef.current?.executeCommand({ subCommand: 'getState' }) as EditableState | undefined;
    if (nextState) {
      updateState(nextState);
    }
  }, [updateState, value]);

  useEffect(() => {
    if (shouldAutoFocusRef.current && editableRef.current) {
      focusEditable(editableRef.current.nodeId);
    }
  }, []);

  useEffect(() => {
    if (!pressed) {
      return;
    }
    const release = () => setPressed(false);
    const removeMouseUp = addEventListener('mouseup', release);
    const removeTouchEnd = addEventListener('touchend', release);
    const removeTouchCancel = addEventListener('touchcancel', release);
    return () => {
      removeMouseUp();
      removeTouchEnd();
      removeTouchCancel();
    };
  }, [pressed]);

  useEffect(() => {
    if (disabled) {
      setHovered(false);
      setPressed(false);
    }
  }, [disabled]);

  useEffect(() => {
    if (!focused) {
      setCaretVisible(false);
      return;
    }
    const expectedDisplayValue = displayValue;
    setCaretVisible(true);
    const timer = setInterval(() => {
      if (stateRef.current.value + stateRef.current.compositionText === expectedDisplayValue) {
        setCaretVisible((visible) => !visible);
      }
    }, blinkInterval ?? 500);
    return () => clearInterval(timer);
  }, [blinkInterval, displayValue, focused]);

  useEffect(() => {
    if (!focused || !caretPosition) {
      return;
    }
    editableRef.current?.executeCommand({
      subCommand: 'setCaretRect',
      x: paddingX + caretPosition[0],
      y: (viewportHeight - caretHeight) / 2,
      width: caretWidth,
      height: caretHeight,
    });
  }, [caretHeight, caretPosition, caretWidth, focused, paddingX, viewportHeight]);

  const caretX = caretPosition?.[0] ?? 0;

  return (
    <editable
      {...nodeProps}
      ref={editableRef}
      value={initialValue}
      disabled={disabled}
      readOnly={readOnly}
      interactive={disabled ? false : interactive}
      cursor={cursor}
      onMouseEnter={mergeEvent(onMouseEnter, () => setHovered(true))}
      onMouseLeave={mergeEvent(onMouseLeave, () => {
        setHovered(false);
        setPressed(false);
      })}
      onMouseDown={mergeEvent(onMouseDown, () => setPressed(true))}
      onMouseUp={mergeEvent(onMouseUp, () => {
        setPressed(false);
        setHovered(true);
      })}
      onTouchStart={mergeEvent(onTouchStart, () => setPressed(true))}
      onTouchEnd={mergeEvent(onTouchEnd, () => {
        setPressed(false);
        setHovered(true);
      })}
      onTouchCancel={mergeEvent(onTouchCancel, () => {
        setHovered(false);
        setPressed(false);
      })}
      onInput={handleInput}
      onChange={handleChange}
      onFocus={handleFocus}
      onBlur={handleBlur}
      onCompositionStart={handleCompositionStart}
      onCompositionUpdate={handleCompositionUpdate}
      onCompositionEnd={handleCompositionEnd}
    >
      {backgroundStyle ? (
        <sprite {...backgroundStyle} targetWidth={viewportWidth} targetHeight={viewportHeight} interactive={false} />
      ) : null}
      <clip x={paddingX} width={contentWidth} height={viewportHeight} interactive={false}>
        <text
          {...textStyle}
          text={displayValue}
          parseMarkup={false}
          printMode="instant"
          anchor={[0, 0.5]}
          pivot={[0, 0.5]}
          interactive={false}
          onTextLayout={handleTextLayout}
        />
        {state.value.length === 0 && state.compositionText.length === 0 && placeholder ? (
          <text
            {...textStyle}
            {...placeholderStyle}
            text={placeholder}
            parseMarkup={false}
            printMode="instant"
            anchor={[0, 0.5]}
            pivot={[0, 0.5]}
            interactive={false}
          />
        ) : null}
        <sprite
          {...caretSpriteProps}
          x={caretX}
          y={(viewportHeight - caretHeight) / 2}
          mode="nineslice"
          bounds={[0, 0, 0, 0]}
          targetWidth={caretWidth}
          targetHeight={caretHeight}
          visible={focused && caretVisible && caretPosition !== undefined}
          interactive={false}
        />
      </clip>
    </editable>
  );
});
