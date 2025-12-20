// DOM / Script Loading Test
console.log('Starting DOM/Script loading test...');

// We'll try to load a script that defines a global variable
const script = document.createElement('script');
script.src =
  'https://gist.githubusercontent.com/Icemic/0918139adb85b0b03acf533f7c450ad2/raw/29116d3a47d85b590d45018a0d991318d9b62b28/simple-test-for-moyu-fetch';

script.onload = function () {
  console.log('✓ Script loaded successfully');
  if (typeof momoyuyu !== 'undefined') {
    console.log('✓ momoyuyu is defined! ', momoyuyu);
    console.log(momoyuyu.foo === 'bar' ? '✓' : '✗', 'momoyuyu.foo === bar ');
  } else {
    console.log('✗ momoyuyu is NOT defined');
  }
};

script.onerror = function (e) {
  console.error('✗ Script load failed');
};

console.log('Appending script to head...');
document.head.appendChild(script);
