import type {
  ControlSpriteProps,
  ControlTextStyle,
  SelectListProps,
  SelectOptionSpriteProps,
  SliderThumbProps,
  SliderTrackProps,
} from '@momoyu-ink/kit';

/**
 * Gallery design system — "Midnight Console".
 *
 * All sprite assets are neutral grayscale (see scripts/generate-assets.mjs);
 * the engine multiplies texel color with `tint`, so white pixels render the
 * tint color exactly and gray pixels become darker shades of the same hue.
 * Every color decision lives here so pages stay consistent.
 */

export const COLOR = {
  /** Page title text. */
  pageTitle: '#f4f7ff',
  /** Page header description. */
  pageDescription: '#8fa0bd',
  /** Panel title gold. */
  panelTitle: '#e8c97a',
  /** Primary body text. */
  text: '#dbe4f3',
  /** Secondary captions and expectation notes. */
  caption: '#8fa0bd',
  /** Tertiary hints. */
  dim: '#5c6a84',
  /** Gold accent for selected states and highlights. */
  accent: '#e8c97a',
  /** Tint for panel chrome (border renders exactly this; fill is a darker shade). */
  panelTint: '#2c3a52',
  /** Tint for interactive controls in idle state. */
  controlTint: '#54688c',
  /** Tint for selected/active controls. */
  controlTintActive: '#3d5a80',
  /** Navigation item text. */
  navText: '#aab6cc',
  /** Solid dark zone used to visualize fixed-size layout areas. */
  zone: '#0d1322',
} as const;

/** Demo item colors, inherited from the original layout-test page. */
export const ITEM_COLORS = ['#4f75c9', '#5f9b72', '#a66b75', '#7d68b5', '#b18448'] as const;

export const TEXT = {
  pageTitle: { fontSize: 40, fillColor: COLOR.pageTitle },
  pageDescription: { fontSize: 20, fillColor: COLOR.pageDescription },
  panelTitle: { fontSize: 24, fillColor: COLOR.panelTitle },
  body: { fontSize: 22, fillColor: COLOR.text },
  caption: { fontSize: 19, fillColor: COLOR.caption },
} as const;

const IMAGES = 'images/';

interface NineSliceSprite {
  mode: 'nineslice';
  bounds: [number, number, number, number];
}

const NINE_SLICE: NineSliceSprite = { mode: 'nineslice', bounds: [0.25, 0.25, 0.25, 0.25] };
const DROPDOWN_NINE_SLICE: NineSliceSprite = { mode: 'nineslice', bounds: [0.48, 0.1, 0.48, 0.1] };
const CHIP_NINE_SLICE: NineSliceSprite = { mode: 'nineslice', bounds: [0.3, 0.3, 0.3, 0.3] };

/** Panel chrome. Sized by the caller via targetWidth/targetHeight. */
export const PANEL_SPRITE = {
  src: `${IMAGES}panel.png`,
  ...NINE_SLICE,
  tint: COLOR.panelTint,
};

/** 1x1 white pixel; tint and stretch for divider lines and accent bars. */
export const PIXEL_SPRITE = `${IMAGES}pixel.png`;

/** Colored demo block (dark fill + border of the given hue). */
export function chipSprite(color: string) {
  return {
    src: `${IMAGES}chip.png`,
    ...CHIP_NINE_SLICE,
    tint: color,
  };
}

export const BUTTON_SPRITE: ControlSpriteProps = {
  src: [`${IMAGES}button.png`, `${IMAGES}button_hover.png`, `${IMAGES}button_press.png`],
  ...NINE_SLICE,
  tint: COLOR.controlTint,
  targetWidth: 280,
  targetHeight: 56,
};

export const BUTTON_TEXT_STYLE: ControlTextStyle = {
  fontSize: 22,
  glyphGridSize: 22,
  fillColor: '#ffffff',
};

export const CHECKBOX_UNCHECKED_SPRITE: ControlSpriteProps = {
  src: [`${IMAGES}unchecked.png`, `${IMAGES}unchecked_hover.png`, `${IMAGES}unchecked_press.png`],
  tint: COLOR.controlTint,
};

export const CHECKBOX_CHECKED_SPRITE: ControlSpriteProps = {
  src: [`${IMAGES}checked.png`, `${IMAGES}checked_hover.png`, `${IMAGES}checked_press.png`],
  tint: COLOR.controlTint,
};

export const SELECT_TRIGGER: ControlSpriteProps = {
  src: [`${IMAGES}dropdown.png`, `${IMAGES}dropdown_hover.png`, `${IMAGES}dropdown_press.png`],
  ...DROPDOWN_NINE_SLICE,
  tint: COLOR.controlTint,
  targetWidth: 360,
  targetHeight: 56,
};

// SelectListProps still extends plain SpriteProps, so this asset remains
// pre-colored by the generator instead of using a runtime tint.

export const SELECT_LIST: SelectListProps = {
  src: `${IMAGES}dropdown_list.png`,
  ...NINE_SLICE,
  targetWidth: 360,
  paddingX: 4,
  paddingY: 4,
};

export const SELECT_OPTION: SelectOptionSpriteProps = {
  src: [
    `${IMAGES}dropdown_listitem.png`,
    `${IMAGES}dropdown_listitem_hover.png`,
    `${IMAGES}dropdown_listitem_press.png`,
  ],
  ...NINE_SLICE,
  tint: COLOR.controlTint,
  targetWidth: 352,
  targetHeight: 50,
};

export const SLIDER_TRACK: SliderTrackProps = {
  src: [`${IMAGES}slider_track.png`, `${IMAGES}slider_track_hover.png`, `${IMAGES}slider_track_press.png`],
  ...NINE_SLICE,
  tint: COLOR.controlTint,
  targetWidth: 360,
  targetHeight: 40,
};

export const SLIDER_THUMB: SliderThumbProps = {
  src: [`${IMAGES}slider_handle.png`, `${IMAGES}slider_handle_hover.png`, `${IMAGES}slider_handle_press.png`],
  ...NINE_SLICE,
  tint: COLOR.controlTint,
  targetWidth: 28,
  targetHeight: 40,
};

/** Solid dark rectangle for visualizing fixed-size layout zones. */
export const ZONE_SPRITE: { src: string; tint: string } = {
  src: PIXEL_SPRITE,
  tint: COLOR.zone,
};
