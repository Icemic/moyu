import React, { useEffect, useState } from 'react';
import { ErrorBoundary } from 'react-error-boundary';

import { createRoot, hai, useTransition } from '@hai/lib';
import { ListButton } from './components/list-button';
import { TextWindow } from './components/textwindow';
import { Dialog } from './components/dialog';

function App() {
  const [showDialog, setShowDialog] = useState(false);
  const [dialogContent, setDialogContent] = useState('');
  const [dialogTitle, setDialogTitle] = useState('');

  const logError = (error: Error, info: { componentStack: string }) => {
    console.error('react error:', JSON.stringify(typeof error));
    console.error(info.componentStack);
  };

  const list = ['项目1', '项目2', '项目3', '项目4', '项目5', '项目6', '项目7'];

  const transitions = useTransition(
    list.map((item, index) => ({ item, index })),
    {
      keys: (item) => item.item,
      from: { opacity: 0, x: -50 },
      enter: (item) => ({ opacity: 1, x: 0, delay: item.index * 80 }),
      leave: { opacity: 0, x: -50 },
    }
  );

  const handleTextWindowButtonClicked = (id: string) => {
    setDialogContent(`点击了${id}`);
    setDialogTitle('提示');
    setShowDialog(true);
  };

  const handleDialogConfirm = (yes?: boolean) => {
    console.info('点击了', yes ? '确定' : '取消');
    setShowDialog(false);
  };

  const handleListButtonClick = (index: number) => {
    if (index === 5) {
      console.log('点击了项目6');
      void (
        hai.executePluginCommand('audio', {
          subCommand: 'load',
          name: 'test',
          src: 'audio/test.ogg',
          autoPlay: false,
        }) as Promise<void>
      ).then(() => {
        console.log('audio loaded');
      });
    } else if (index === 6) {
      console.log('点击了项目7');
      void hai.executePluginCommand('audio', {
        subCommand: 'setVolume',
        name: 'test',
        volume: 1.0,
      });
      void hai.executePluginCommand('audio', {
        subCommand: 'play',
        name: 'test',
      });
    }
  };

  return (
    <ErrorBoundary FallbackComponent={ErrorFallback} onError={logError}>
      <container label="App">
        <sprite label="背景图" src="classroom1.png" scale={1280 / 1344} />
        <TextWindow onItemClicked={handleTextWindowButtonClicked} />
        <Dialog
          show={showDialog}
          title={dialogTitle}
          content={dialogContent}
          mode="confirm"
          onConfirm={handleDialogConfirm}
        />
        <container label="列表容器" x={0} y={0}>
          <sprite label="列表底纹" src="mask.png" scaleX={200} scaleY={420} />
          {transitions((style, { item, index }) => (
            <ListButton
              style={style}
              label={`item-${index}`}
              title={item}
              index={index}
              onClick={handleListButtonClick}
            />
          ))}
        </container>
      </container>
    </ErrorBoundary>
  );
}

function ErrorFallback({ error, resetErrorBoundary }) {
  // Call resetErrorBoundary() to reset the error boundary and retry the render.

  return (
    <container>
      <sprite label="背景图" src="classroom1.png" scale={1280 / 1344} />
    </container>
  );
}

const root = createRoot();

root.render(<App />);
