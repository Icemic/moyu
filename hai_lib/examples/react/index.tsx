import React, { useEffect, useState } from 'react';
import { createRoot } from '../../src';

function App() {
  const [src, setSrc] = useState('title.png');
  useEffect(() => {
    setTimeout(() => {
      setSrc('button_n_02.png');
    }, 2000);
  }, []);
  return <sprite label="title" src={src} />;
}

const root = createRoot();

root.render(<App />);
