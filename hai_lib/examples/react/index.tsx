import React from 'react';
import { createRoot } from '../../src';

function App() {
  return <sprite label="title" src="title.png" />;
}

const root = createRoot();

root.render(<App />);
