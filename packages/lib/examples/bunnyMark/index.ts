import { hai } from '../..';
import BunnyMark from './bunnyMark';

const bunnies = [
  'bunny/rabbitv3_ash.png',
  'bunny/rabbitv3_batman.png',
  'bunny/rabbitv3_bb8.png',
  'bunny/rabbitv3_frankenstein.png',
  'bunny/rabbitv3_neo.png',
  'bunny/rabbitv3_sonic.png',
  'bunny/rabbitv3_spidey.png',
  'bunny/rabbitv3_stormtrooper.png',
  'bunny/rabbitv3_superman.png',
  'bunny/rabbitv3_tron.png',
  'bunny/rabbitv3_wolverine.png',
  'bunny/rabbitv3.png',
];

// bunny size is 25x32 with anchor [0.5, 1.0]
const bunnyMark = new BunnyMark(bunnies, {
  left: 12,
  right: 1280 - 12,
  top: 32,
  bottom: 720,
});

bunnyMark.addBunny(200);

hai.resizeWindow(800, 600);

// setInterval(() => hai.loadResources(), 500);

let i = 0;
let cc = 0;

let t = Date.now();
const loop = () => {
  setTimeout(loop, 16);
  bunnyMark.update();
  const t2 = Date.now();
  // console.info(t2 - t);
  cc += t2 - t;
  i += 1;
  console.info((cc / i) << 0);
  t = t2;
};

setInterval(() => {
  bunnyMark.addBunny(10);
}, 1600);

loop();
