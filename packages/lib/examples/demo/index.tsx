import React from 'react';
import { createRoot } from '../..';
import { ListButton } from './components/list-button';

function App() {
  const list = ['项目1', '项目2', '项目3', '项目4', '项目5', '项目6', '项目7'];
  // const list = ['项目1'];

  return (
    <container label="App">
      <sprite label="背景图" src="classroom1.png" scale={1280 / 1344} />
      <container label="列表容器" x={0} y={0}>
        <sprite label="列表底纹" src="mask.png" scaleX={200} scaleY={720} />
        {list.map((item, index) => (
          <ListButton label={`item-${index}`} title={item} index={index} />
        ))}
      </container>
    </container>
  );
}

const root = createRoot();

root.render(<App />);
