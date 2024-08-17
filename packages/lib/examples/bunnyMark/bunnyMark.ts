import Bunny, { Bounds } from './bunny';
import { hai } from '../..';

export default class BunnyMark {
  count = 0;
  bunnies: Bunny[] = [];
  textures: string[] = [];
  bounds: Bounds;
  lastTime = Date.now();
  constructor(textures: string[], bounds: Bounds) {
    this.textures = textures;
    this.bounds = bounds;
  }

  addBunny(num: number) {
    for (let i = 0; i < num; i++) {
      const texture = this.textures[this.count % this.textures.length];

      const id = hai.createInstance('sprite', '', {
        src: texture,
        pivot: [0.5, 1.0],
        anchor: [0.5, 1.0],
      });

      const bunny = new Bunny(id, this.bounds);
      bunny.position.x = this.count % 2 === 0 ? this.bounds.left : this.bounds.right;
      this.bunnies.push(bunny);

      hai.addChild(0, id);

      this.count++;
    }
  }

  update() {
    const time = Date.now();
    const deltaFrame = ((time - this.lastTime) / 1000) * 60;
    for (const bunny of this.bunnies) {
      bunny.update(deltaFrame);
    }
    this.lastTime = time;
  }
}
