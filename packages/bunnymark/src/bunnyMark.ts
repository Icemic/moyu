import Bunny, { type Bounds } from './bunny';
import { moyu } from '@momoyu-ink/kit';

export default class BunnyMark {
  public count = 0;
  public bunnies: Bunny[] = [];
  public textures: string[] = [];
  public bounds: Bounds;
  public lastTime = Date.now();
  public constructor(textures: string[], bounds: Bounds) {
    this.textures = textures;
    this.bounds = bounds;
  }

  public addBunny(num: number) {
    for (let i = 0; i < num; i++) {
      const texture = this.textures[this.count % this.textures.length];

      const id = moyu.createInstance('sprite', '', {
        src: texture,
        pivot: [0.5, 1.0],
        anchor: [0.5, 1.0],
      });

      const bunny = new Bunny(id, this.bounds);
      bunny.position.x = this.count % 2 === 0 ? this.bounds.left : this.bounds.right;
      this.bunnies.push(bunny);

      moyu.addChild(0, id);

      this.count++;
    }
  }

  public update() {
    const time = Date.now();
    const deltaFrame = ((time - this.lastTime) / 1000) * 60;
    for (const bunny of this.bunnies) {
      bunny.update(deltaFrame);
    }
    this.lastTime = time;
  }
}
