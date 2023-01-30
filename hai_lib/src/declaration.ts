export type DetailedHaiProps<E extends HaiNodeLikeAttributes> = E;

export interface HaiNodeLikeAttributes {
  label?: string;
}

export type HaiNodeAttributes = HaiNodeLikeAttributes;
export interface HaiSpriteAttribute extends HaiNodeLikeAttributes {
  src: string;
}

declare global {
  namespace JSX {
    interface IntrinsicElements {
      node: DetailedHaiProps<HaiNodeAttributes>;
      sprite: DetailedHaiProps<HaiSpriteAttribute>;
    }
  }
}
