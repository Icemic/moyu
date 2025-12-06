import type { Meta, StoryObj } from '@storybook/react';
import { Text } from './Text';
import { fn } from 'storybook/test';

const meta = {
  title: 'Base/Text',
  component: Text,
  parameters: {
    layout: 'centered',
  },
  argTypes: {
    text: { control: 'text' },
    fontSize: { control: { type: 'range', min: 12, max: 72, step: 1 } },
    fillColor: { control: 'color' },
    printMode: {
      control: 'select',
      options: ['instant', 'typewriter', 'printer'],
    },
    printSpeed: { control: { type: 'range', min: 1, max: 200, step: 1 } },
    boxWidth: { control: { type: 'range', min: 100, max: 1200, step: 10 } },
    boxHeight: { control: { type: 'range', min: 50, max: 600, step: 10 } },
    lineHeight: { control: { type: 'range', min: 1, max: 3, step: 0.1 } },
    interactive: { control: 'boolean' },
    cursor: {
      control: 'select',
      options: ['default', 'pointer', 'text'],
    },
    onClick: { action: 'clicked' },
    onMouseEnter: { action: 'mouse entered' },
    onMouseLeave: { action: 'mouse left' },
    onStart: { action: 'printing started' },
    onFinish: { action: 'printing finished' },
    onProgress: { action: 'printing progress' },
  },
} satisfies Meta<typeof Text>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    text: 'Hello, Moyu Framework!',
    fontSize: 24,
    fillColor: '#FFFFFF',
    printMode: 'instant',
  },
};

export const Typewriter: Story = {
  args: {
    text: 'This text appears character by character...',
    fontSize: 20,
    fillColor: '#FFFFFF',
    printMode: 'typewriter',
    printSpeed: 50,
    onStart: fn(),
    onFinish: fn(),
    onProgress: fn(),
  },
};

export const LargeText: Story = {
  args: {
    text: 'Large sized text',
    fontSize: 48,
    fillColor: '#FFD700',
    printMode: 'instant',
  },
};

export const Paragraph: Story = {
  args: {
    text: 'This is a longer paragraph of text to demonstrate how the text component handles multiple lines. The text will wrap according to the boxWidth property.',
    fontSize: 18,
    fillColor: '#FFFFFF',
    printMode: 'instant',
    boxWidth: 600,
    lineHeight: 1.8,
  },
};

export const Interactive: Story = {
  args: {
    text: 'Click me!',
    fontSize: 32,
    fillColor: '#4ECDC4',
    printMode: 'instant',
    interactive: true,
    cursor: 'pointer',
    onClick: fn(),
    onMouseEnter: fn(),
    onMouseLeave: fn(),
  },
};

export const TypewriterWithEvents: Story = {
  args: {
    text: 'Watch the Actions panel as this text types out...',
    fontSize: 20,
    fillColor: '#FFFFFF',
    printMode: 'typewriter',
    printSpeed: 30,
    onStart: fn(),
    onFinish: fn(),
    onProgress: fn(),
  },
};
