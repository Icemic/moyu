import Bunny, { Bounds } from './bunny';
import { hai } from '../..';

export default class BunnyMark {
  count = 0;
  bunnies: Bunny[] = [];
  textures: string[] = [];
  bounds: Bounds;
  constructor(textures: string[], bounds: Bounds) {
    this.textures = textures;
    this.bounds = bounds;
  }

  addBunny(num: number) {
    for (let i = 0; i < num; i++) {
      const texture = this.textures[this.count % this.textures.length];

      const id = hai.createInstance('sprite', '', {
        src: texture,
      });

      const bunny = new Bunny(id, this.bounds);
      bunny.position.x = (this.count % 2) * 800;
      this.bunnies.push(bunny);

      hai.addChild(0, id);

      this.count++;
    }
  }

  update() {
    for (const bunny of this.bunnies) {
      bunny.update();
    }
  }
}
