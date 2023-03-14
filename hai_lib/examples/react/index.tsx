import React, { useEffect, useState } from 'react';
import { createRoot } from '../..';

function App() {
  const [src, setSrc] = useState('title.png');
  useEffect(() => {
    setTimeout(() => {
      setSrc('button_n_02.png');
    }, 2000);
  }, []);
  //
  return (
    <container>
      <sprite label="title" src={src} />
      <video label="video" src={'sample.mp4'} scale={0.5} x={0} />
    </container>
  );
}

const root = createRoot();

root.render(<App />);
