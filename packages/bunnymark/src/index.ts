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

bunnyMark.addBunny(8000);

let frame_count = 0;
let last_fps_time = 0;

const loop: FrameRequestCallback = (now) => {
  bunnyMark.update();

  frame_count++;

  if (now - last_fps_time > 1000) {
    console.log('fps(script):', frame_count);
    frame_count = 0;
    last_fps_time = now;
  }

  requestAnimationFrame(loop);
};

// setInterval(() => {
//   bunnyMark.addBunny(10);
// }, 1600);

requestAnimationFrame(loop);
