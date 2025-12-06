import React from 'react';
import { Cursor } from '../src/declaration';

export interface TextProps {
  /** Text content to display */
  text?: string;
  /** Font size */
  fontSize?: number;
  /** Text color */
  fillColor?: string;
  /** Print mode */
  printMode?: 'instant' | 'typewriter' | 'printer';
  /** Print speed (for typewriter/printer mode) */
  printSpeed?: number;
  /** Box width */
  boxWidth?: number;
  /** Box height */
  boxHeight?: number;
  /** Line height */
  lineHeight?: number;
  /** Interactive (enables mouse events) */
  interactive?: boolean;
  /** Cursor style when hovering */
  cursor?: Cursor;
  /** Click event handler */
  onClick?: () => void;
  /** Mouse enter event handler */
  onMouseEnter?: () => void;
  /** Mouse leave event handler */
  onMouseLeave?: () => void;
  /** Callback when printing starts */
  onStart?: () => void;
  /** Callback when printing finishes */
  onFinish?: () => void;
  /** Callback for printing progress (0-1) */
  onProgress?: (progress: number) => void;
}

/**
 * Basic Moyu Text Component for demonstration
 * Uses the <text> element from @momoyu-ink/kit
 */
export const Text = ({
  text = 'Sample Text',
  fontSize = 24,
  fillColor = '#FFFFFF',
  printMode = 'instant',
  printSpeed = 50,
  boxWidth = 800,
  boxHeight = 200,
  lineHeight = 1.5,
  interactive = false,
  cursor = 'pointer',
  onClick,
  onMouseEnter,
  onMouseLeave,
  onStart,
  onFinish,
  onProgress,
}: TextProps) => {
  return (
    <text
      text={text}
      fontSize={fontSize}
      fillColor={fillColor}
      printMode={printMode}
      printSpeed={printSpeed}
      boxWidth={boxWidth}
      boxHeight={boxHeight}
      lineHeight={lineHeight}
      interactive={interactive}
      cursor={cursor}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onStart={onStart}
      onFinish={onFinish}
      onProgress={onProgress}
    />
  );
};
