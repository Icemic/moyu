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

  const text = `人人生而自由，在尊严和权利上一律平等。他们赋有理性和良心，并应以兄弟关系的精神相对待。
人人有资格享有本宣言所载的一切权利和自由，不分种族、肤色、性别、语言、宗教、政治或其他见解、国籍或社会出身、财产、出生或其他身分等任何区别。并且不得因一人所属的国家或领土的政治的、行政的或者国际的地位之不同而有所区别，无论该领土是独立领土、托管领土、非自治领土或者处于其他任何主权受限制的情况之下。`;

  return (
    <container x={50} y={100}>
      <sprite label="背景图" src={src} rotation={0.2} />
      {/* <video label="video" src={'D:\\Workspace\\epic-rs\\output\\video.mp4'} scale={0.5} x={0} mode="stream" /> */}
      <text
        label="文本"
        text={text}
        layoutStyle={{
          direction: 'horizontal' as const,
          boxWidth: 800,
          boxHeight: 720,
          glyphGridSize: 24,
        }}
        rotation={0}
        textStyle={{
          fontSize: 24,
          lineHeight: 1.5,
          fillColor: 'black',
          indent: 0,
          stroke: {},
          shadow: {},
        }}
        x={50}
        y={100}
      />
    </container>
  );
}

const root = createRoot();

root.render(<App />);
