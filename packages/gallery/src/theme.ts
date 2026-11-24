/**
 * Gallery design system — "Blueprint".
 *
 * Colours live in `theme/tokens.json`, the palette and sprite builders in `theme/themes.ts`;
 * this module is the public entry every page imports (`../theme`), so those imports keep
 * working unchanged while the configuration moved into its own files.
 */
import type {
  ControlSpriteProps,
  ControlTextStyle,
  InputBackground,
  InputCaretStyle,
  SelectListProps,
  SelectOptionSpriteProps,
  SliderThumbProps,
  SliderTrackProps,
} from '@momoyu-ink/kit';
import {
  controlTextStyle,
  type GalleryMode,
  type GalleryPalette,
  ITEM_COLORS,
  type NineSliceSprite,
  TYPOGRAPHY,
  themeOf,
} from './theme/themes';

/**
 * Gallery design system — "Blueprint".
 *
 * Small radii, hairline cards and one green accent, in a light and a dark appearance.
 * Colours come from `tokens.json`; sprites are baked per mode by
 * `scripts/generate-assets.mjs` and only neutral assets (`pixel`, `chip`) are tinted at
 * runtime.
 *
 * The exports below are live singletons: `applyGalleryMode()` rewrites them in place, so
 * a page reads the current mode's values whenever it renders. Anything that captures a
 * value at module scope must use a getter (see `GROUP_LABEL` in the pages), otherwise it
 * would freeze the mode that happened to be active at import time.
 */

export { type GalleryMode, ITEM_COLORS };

interface ColorTokens {
  pageTitle: string;
  pageDescription: string;
  panelTitle: string;
  text: string;
  caption: string;
  dim: string;
  accent: string;
  /** Card surface: the sidebar column and any surface that sits on the page. */
  surface: string;
  /** Muted surface, for tinted blocks that sit next to cards. */
  panelTint: string;
  /** Neutral chip tint, for scrollbars and stand-in surfaces. */
  controlTint: string;
  navText: string;
  zone: string;
}

interface TextTokens {
  pageTitle: ControlTextStyle;
  pageDescription: ControlTextStyle;
  panelTitle: ControlTextStyle;
  body: ControlTextStyle;
  caption: ControlTextStyle;
}

function colorTokens(palette: GalleryPalette): ColorTokens {
  return {
    pageTitle: palette.text,
    pageDescription: palette.textSecondary,
    panelTitle: palette.text,
    text: palette.text,
    caption: palette.textSecondary,
    dim: palette.textTertiary,
    accent: palette.accent,
    surface: palette.surface,
    panelTint: palette.surfaceMuted,
    controlTint: palette.textTertiary,
    navText: palette.textSecondary,
    zone: palette.zone,
  };
}

function textTokens(palette: GalleryPalette): TextTokens {
  return {
    pageTitle: { fontSize: TYPOGRAPHY.pageTitle, fontWeight: 600, fillColor: palette.text },
    pageDescription: { fontSize: TYPOGRAPHY.pageDescription, fillColor: palette.textSecondary },
    panelTitle: { fontSize: TYPOGRAPHY.panelTitle, fontWeight: 600, fillColor: palette.text },
    body: { fontSize: TYPOGRAPHY.body, fillColor: palette.text },
    caption: { fontSize: TYPOGRAPHY.caption, fillColor: palette.textSecondary },
  };
}

let activeMode: GalleryMode = 'light';
let theme = themeOf(activeMode);

/** 1x1 white pixel; tint and stretch for dividers, accent bars and solid fills. */
export const PIXEL_SPRITE = 'images/pixel.png';

/** Neutral chip (flat, tintable) for demo blocks that carry an arbitrary colour. */
export function chipSprite(color: string) {
  return {
    src: 'images/chip.png',
    mode: 'nineslice' as const,
    bounds: [0.3, 0.3, 0.3, 0.3] as [number, number, number, number],
    tint: color,
  };
}

export const COLOR: ColorTokens = colorTokens(theme.palette);
export const TEXT: TextTokens = textTokens(theme.palette);

export const PANEL_SPRITE: NineSliceSprite = { ...theme.sprites.panel };
export const BUTTON_SPRITE: ControlSpriteProps = { ...theme.sprites.button };
export const TAB_SPRITE: ControlSpriteProps = { ...theme.sprites.tab };
export const BUTTON_SELECTED_SPRITE: ControlSpriteProps = { ...theme.sprites.buttonSelected };
export const PRIMARY_BUTTON_SPRITE: ControlSpriteProps = { ...theme.sprites.primaryButton };
export const CHECKBOX_UNCHECKED_SPRITE: ControlSpriteProps = { ...theme.sprites.checkboxUnchecked };
export const CHECKBOX_CHECKED_SPRITE: ControlSpriteProps = { ...theme.sprites.checkboxChecked };
export const RADIO_UNCHECKED_SPRITE: ControlSpriteProps = { ...theme.sprites.radioUnchecked };
export const RADIO_CHECKED_SPRITE: ControlSpriteProps = { ...theme.sprites.radioChecked };
export const SELECT_TRIGGER: ControlSpriteProps = {
  ...theme.sprites.selectTrigger,
  targetWidth: 360,
  targetHeight: 56,
};
export const SELECT_LIST: SelectListProps = { ...theme.sprites.selectList, targetWidth: 360 };
export const SELECT_OPTION: SelectOptionSpriteProps = {
  ...theme.sprites.selectOption,
  targetWidth: 352,
  targetHeight: 50,
};
export const SLIDER_TRACK: SliderTrackProps = {
  ...theme.sprites.sliderTrack,
  targetWidth: 360,
  targetHeight: 40,
};
export const SLIDER_THUMB: SliderThumbProps = { ...theme.sprites.sliderThumb, targetWidth: 40, targetHeight: 40 };
export const INPUT_BACKGROUND: InputBackground = { ...theme.sprites.input };
export const INPUT_CARET: InputCaretStyle = { ...theme.sprites.caret };
export const ZONE_SPRITE: { src: string; tint: string } = { src: PIXEL_SPRITE, tint: COLOR.zone };

export const BUTTON_TEXT_STYLE: ControlTextStyle = controlTextStyle(theme.palette);

/**
 * A live view of the default button at a fixed size.
 *
 * Pages keep these at module scope, so the fields must read through to the singletons
 * instead of copying them; otherwise the size survives a mode switch but the sprite does
 * not.
 */
export function sizedButton(targetWidth: number, targetHeight: number): ControlSpriteProps {
  return {
    get src() {
      return BUTTON_SPRITE.src;
    },
    get mode() {
      return BUTTON_SPRITE.mode;
    },
    get bounds() {
      return BUTTON_SPRITE.bounds;
    },
    targetWidth,
    targetHeight,
  };
}

/** Mode the gallery is currently rendered in. */
export function currentGalleryMode(): GalleryMode {
  return activeMode;
}

/** Page background for a mode. */
export function backgroundSprite(mode: GalleryMode = activeMode) {
  return `images/${mode}/bg.png`;
}

/**
 * Switch appearance. Rewrites the exported singletons in place so pages see the new
 * values on their next render; callers must re-render afterwards.
 */
export function applyGalleryMode(mode: GalleryMode) {
  activeMode = mode;
  theme = themeOf(mode);

  Object.assign(COLOR, colorTokens(theme.palette));
  Object.assign(TEXT, textTokens(theme.palette));
  Object.assign(BUTTON_TEXT_STYLE, controlTextStyle(theme.palette));
  Object.assign(ZONE_SPRITE, { tint: COLOR.zone });

  Object.assign(PANEL_SPRITE, theme.sprites.panel);
  Object.assign(BUTTON_SPRITE, theme.sprites.button);
  Object.assign(TAB_SPRITE, theme.sprites.tab);
  Object.assign(BUTTON_SELECTED_SPRITE, theme.sprites.buttonSelected);
  Object.assign(PRIMARY_BUTTON_SPRITE, theme.sprites.primaryButton);
  Object.assign(CHECKBOX_UNCHECKED_SPRITE, theme.sprites.checkboxUnchecked);
  Object.assign(CHECKBOX_CHECKED_SPRITE, theme.sprites.checkboxChecked);
  Object.assign(RADIO_UNCHECKED_SPRITE, theme.sprites.radioUnchecked);
  Object.assign(RADIO_CHECKED_SPRITE, theme.sprites.radioChecked);
  Object.assign(SELECT_TRIGGER, { ...theme.sprites.selectTrigger, targetWidth: 360, targetHeight: 56 });
  Object.assign(SELECT_LIST, { ...theme.sprites.selectList, targetWidth: 360 });
  Object.assign(SELECT_OPTION, { ...theme.sprites.selectOption, targetWidth: 352, targetHeight: 50 });
  Object.assign(SLIDER_TRACK, { ...theme.sprites.sliderTrack, targetWidth: 360, targetHeight: 40 });
  Object.assign(SLIDER_THUMB, { ...theme.sprites.sliderThumb, targetWidth: 40, targetHeight: 40 });
  Object.assign(INPUT_BACKGROUND, theme.sprites.input);
  Object.assign(INPUT_CARET, theme.sprites.caret);
}
