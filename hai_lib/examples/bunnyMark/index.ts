import * as hai from '../../';
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

const bunnyMark = new BunnyMark(bunnies, {
  left: 0,
  right: 800,
  top: 0,
  bottom: 600,
});

bunnyMark.addBunny(50);

hai.resizeWindow(800, 600);

const loop = () => {
  bunnyMark.update();
  setTimeout(loop, 16);
};

setInterval(() => {
  bunnyMark.addBunny(10);
  hai.loadResources();
}, 800);

loop();
