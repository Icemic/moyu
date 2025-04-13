import { moyu } from '@momoyu-ink/kit';

export interface Bounds {
  left: number;
  right: number;
  top: number;
  bottom: number;
}

export default class Bunny {
  public id: number;
  public gravity: number;
  public speedX: number;
  public speedY: number;
  public position = {
    x: 0,
    y: 0,
  };
  public bounds: Bounds = {
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
  };
  public constructor(id: number, bounds: Bounds) {
    this.id = id;
    /**
     * The amount of gravity
     * @type {Number}
     */
    this.gravity = 0.75;

    /**
     * Horizontal speed
     * @type {Number}
     */
    this.speedX = Math.random() * 10;

    /**
     * Vertical speed
     * @type {Number}
     */
    this.speedY = Math.random() * 10 - 5;

    /**
     * Reference to the bounds object
     * @type {Object}
     */
    this.bounds = bounds;

    // Set the anchor position
    //   this.anchor.x = 0.5;
    //   this.anchor.y = 1;
  }
  /**
   * Update the position of the bunny
   * @method update
   */
  public update(deltaFrame: number) {
    this.position.x += this.speedX * deltaFrame;
    this.position.y += this.speedY * deltaFrame;
    this.speedY += this.gravity * deltaFrame;

    if (this.position.x > this.bounds.right) {
      this.speedX *= -1;
      this.position.x = this.bounds.right;
    } else if (this.position.x < this.bounds.left) {
      this.speedX *= -1;
      this.position.x = this.bounds.left;
    }

    if (this.position.y > this.bounds.bottom) {
      this.speedY *= -0.85;
      this.position.y = this.bounds.bottom;
      if (Math.random() > 0.5) {
        this.speedY -= Math.random() * 6;
      }
    } else if (this.position.y < this.bounds.top) {
      this.speedY = 0;
      this.position.y = this.bounds.top;
    }

    moyu.updateProps(this.id, {
      x: this.position.x,
      y: this.position.y,
    });
  }

  /**
   * Don't use after this.
   * @method destroy
   */
  public destroy() {
    moyu.removeChild(0, this.id);
  }
}
