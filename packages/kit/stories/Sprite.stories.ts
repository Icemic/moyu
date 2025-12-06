import type { Meta, StoryObj } from '@storybook/react';
import { Sprite } from './Sprite';
import { fn } from 'storybook/test';

const meta = {
  title: 'Base/Sprite',
  component: Sprite,
  parameters: {
    layout: 'centered',
  },
  argTypes: {
    src: { control: 'text' },
    x: { control: { type: 'range', min: 0, max: 600, step: 1 } },
    y: { control: { type: 'range', min: 0, max: 400, step: 1 } },
    scale: { control: { type: 'range', min: 0.1, max: 3, step: 0.1, defaultValue: 1 } },
    scaleX: { control: { type: 'range', min: 0.1, max: 3, step: 0.1, defaultValue: 1 } },
    scaleY: { control: { type: 'range', min: 0.1, max: 3, step: 0.1, defaultValue: 1 } },
    rotation: { control: { type: 'range', min: 0, max: Math.PI * 2, step: 0.01 } },
    opacity: { control: { type: 'range', min: 0, max: 1, step: 0.1, defaultValue: 1 } },
    tint: { control: 'color' },
    visible: { control: 'boolean' },
    interactive: { control: 'boolean' },
    cursor: {
      control: 'select',
      options: ['default', 'pointer', 'move', 'grab', 'not-allowed'],
    },
    onClick: { action: 'clicked' },
    onMouseEnter: { action: 'mouse entered' },
    onMouseLeave: { action: 'mouse left' },
  },
} satisfies Meta<typeof Sprite>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    src: 'assets.png',
    x: 300,
    y: 200,
  },
};

export const Scaled: Story = {
  args: {
    src: 'assets.png',
    x: 300,
    y: 200,
    scale: 1.5,
  },
};

export const Rotated: Story = {
  args: {
    src: 'assets.png',
    x: 300,
    y: 200,
    rotation: 45,
  },
};

export const WithTint: Story = {
  args: {
    src: 'assets.png',
    x: 300,
    y: 200,
    tint: '#FF6B6B',
  },
};

export const SemiTransparent: Story = {
  args: {
    src: 'assets.png',
    x: 300,
    y: 200,
    opacity: 0.5,
  },
};

export const Interactive: Story = {
  args: {
    src: 'assets.png',
    x: 300,
    y: 200,
    interactive: true,
    cursor: 'pointer',
    onClick: fn(),
    onMouseEnter: fn(),
    onMouseLeave: fn(),
  },
};

export const CenteredAnchor: Story = {
  args: {
    src: 'assets.png',
    x: 300,
    y: 200,
    anchor: [0.5, 0.5],
    rotation: 45,
  },
};

export const NonUniformScale: Story = {
  args: {
    src: 'assets.png',
    x: 300,
    y: 200,
    scaleX: 2,
    scaleY: 0.5,
  },
};
