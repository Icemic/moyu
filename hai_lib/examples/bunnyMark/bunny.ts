import { hai } from '../..';

export class Bounds {
  left: number;
  right: number;
  top: number;
  bottom: number;
}

export default class Bunny {
  id: number;
  gravity: number;
  speedX: number;
  speedY: number;
  position = {
    x: 0,
    y: 0,
  };
  bounds: Bounds = {
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
  };
  constructor(id: number, bounds: Bounds) {
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
  update() {
    this.position.x += this.speedX;
    this.position.y += this.speedY;
    this.speedY += this.gravity;

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

    hai.moveTo(this.id, this.position.x, this.position.y);
  }

  /**
   * Don't use after this.
   * @method destroy
   */
  destroy() {
    hai.removeChild(0, this.id);
  }
}
