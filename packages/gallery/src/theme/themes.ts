import type {
  ControlSpriteProps,
  ControlStateValue,
  ControlTextStyle,
  InputBackground,
  InputCaretStyle,
  SelectListProps,
  SelectOptionSpriteProps,
  SliderThumbProps,
  SliderTrackProps,
} from '@momoyu-ink/kit';
import tokens from './tokens.json';

/**
 * Theme tokens for one appearance mode.
 *
 * `tokens.json` is the only place colours are written down: the runtime theme and
 * `scripts/generate-assets.mjs` both read it, so a baked sprite and the colour a page
 * draws next to it can never drift apart.
 *
 * The palette follows `../moyu-docs/src/styles/theme-tokens.css`; the shape language
 * (borderless surfaces, small radii) follows the direction chosen during the redesign.
 */

export type GalleryMode = 'light' | 'dark';

export interface GalleryPalette {
  background: string;
  surface: string;
  surfaceMuted: string;
  text: string;
  textSecondary: string;
  textTertiary: string;
  border: string;
  controlBorder: string;
  controlBorderHover: string;
  controlIdle: string;
  controlHover: string;
  controlPress: string;
  accent: string;
  accentHover: string;
  accentPress: string;
  onAccent: string;
  accentSoft: string;
  track: string;
  zone: string;
}

/** A plain sprite config: the panel has no state variants. */
export interface NineSliceSprite {
  src: string;
  mode: 'nineslice';
  bounds: [number, number, number, number];
}

export interface GallerySprites {
  panel: NineSliceSprite;
  button: ControlSpriteProps;
  /** Tab row resting on the page surface: filled with the surface colour, not a control grey. */
  tab: ControlSpriteProps;
  buttonSelected: ControlSpriteProps;
  primaryButton: ControlSpriteProps;
  checkboxUnchecked: ControlSpriteProps;
  checkboxChecked: ControlSpriteProps;
  radioUnchecked: ControlSpriteProps;
  radioChecked: ControlSpriteProps;
  selectTrigger: ControlSpriteProps;
  selectList: SelectListProps;
  selectOption: SelectOptionSpriteProps;
  sliderTrack: SliderTrackProps;
  sliderThumb: SliderThumbProps;
  input: InputBackground;
  caret: InputCaretStyle;
}

export interface GalleryTheme {
  mode: GalleryMode;
  palette: GalleryPalette;
  sprites: GallerySprites;
}

/** Nine-slice bounds for a 64px control sprite: 16px corners keep small radii crisp. */
const CONTROL_BOUNDS: [number, number, number, number] = [0.25, 0.25, 0.25, 0.25];
/** The chevron sits in the right slice, which must not stretch. */
const TRIGGER_BOUNDS: [number, number, number, number] = [0.48, 0.1, 0.48, 0.1];
/** A flat bar stretches on both axes; the caps keep the rounded ends. */
const TRACK_BOUNDS: [number, number, number, number] = [0.3, 0, 0.3, 0];

function controlSprite(
  src: ControlStateValue<string>,
  targetWidth?: number,
  targetHeight?: number,
): ControlSpriteProps {
  return {
    src,
    mode: 'nineslice',
    bounds: CONTROL_BOUNDS,
    targetWidth,
    targetHeight,
  };
}

/** Suffixes the asset generator writes for a control's states. */
const STATES = ['idle', 'hover', 'press'] as const;

/** A stateless nine-slice sprite: panels and input surfaces. */
function panelLike(src: string): NineSliceSprite {
  return { src, mode: 'nineslice', bounds: CONTROL_BOUNDS };
}

/** `images/<mode>/<prefix>_{idle,hover,press}.png`, as a control state tuple. */
function stateSources(base: string, prefix: string): [string, string, string] {
  return STATES.map((state) => `${base}${prefix}_${state}.png`) as [string, string, string];
}

export function paletteOf(mode: GalleryMode): GalleryPalette {
  return tokens.modes[mode] as GalleryPalette;
}

export function themeOf(mode: GalleryMode): GalleryTheme {
  const palette = paletteOf(mode);
  const base = `images/${mode}/`;

  return {
    mode,
    palette,
    sprites: {
      panel: panelLike(`${base}panel.png`),
      button: controlSprite(stateSources(base, 'button')),
      tab: controlSprite(stateSources(base, 'tab')),
      buttonSelected: controlSprite([`${base}button_selected.png`, `${base}button_selected_hover.png`]),
      primaryButton: controlSprite(stateSources(base, 'primary')),
      checkboxUnchecked: { src: [`${base}checkbox.png`, `${base}checkbox_hover.png`] },
      checkboxChecked: { src: [`${base}checkbox_checked.png`, `${base}checkbox_checked_hover.png`] },
      radioUnchecked: { src: [`${base}radio.png`, `${base}radio_hover.png`] },
      radioChecked: { src: [`${base}radio_checked.png`, `${base}radio_checked_hover.png`] },
      selectTrigger: {
        ...controlSprite(stateSources(base, 'select')),
        bounds: TRIGGER_BOUNDS,
      },
      selectList: {
        src: `${base}select_list.png`,
        mode: 'nineslice',
        bounds: CONTROL_BOUNDS,
        paddingX: 4,
        paddingY: 4,
      },
      selectOption: {
        src: stateSources(base, 'select_item'),
        mode: 'nineslice',
        bounds: CONTROL_BOUNDS,
        targetHeight: 50,
      },
      sliderTrack: {
        src: stateSources(base, 'track'),
        mode: 'nineslice',
        bounds: TRACK_BOUNDS,
        targetWidth: 360,
        targetHeight: 40,
      },
      sliderThumb: {
        src: stateSources(base, 'thumb'),
        mode: 'nineslice',
        bounds: CONTROL_BOUNDS,
        targetWidth: 40,
        targetHeight: 40,
      },
      input: {
        idle: panelLike(`${base}input_idle.png`),
        hover: panelLike(`${base}input_hover.png`),
        press: panelLike(`${base}input_press.png`),
        focused: panelLike(`${base}input_focused.png`),
        readOnly: panelLike(`${base}input_readonly.png`),
        disabled: panelLike(`${base}input_disabled.png`),
      },
      caret: {
        src: 'images/pixel.png',
        tint: palette.accent,
        width: 3,
        height: 36,
        blinkInterval: 500,
      },
    },
  };
}

export const TYPOGRAPHY = tokens.type;
export const ITEM_COLORS = tokens.itemColors;

/** Text style shared by every labelled control; pages override sizes as needed. */
export function controlTextStyle(palette: GalleryPalette): ControlTextStyle {
  return {
    fontSize: tokens.type.label,
    fontWeight: 500,
    fillColor: palette.text,
  };
}
