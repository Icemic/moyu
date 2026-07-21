import {
  addEventListener,
  animated,
  Button,
  getStageSize,
  type MouseEvent,
  ScrollView,
  type TouchEvent,
  useScrollView,
} from '@momoyu-ink/kit';
import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType } from 'react';
import { BackdropsPage } from './pages/backdrops';
import { ControlsPage } from './pages/controls';
import { FiltersPage } from './pages/filters';
import { LayoutsPage } from './pages/layouts';
import { PrimitivesPage } from './pages/primitives';
import { RawShadersPage } from './pages/raw-shaders';
import { ShaderTransitionsPage } from './pages/shader-transitions';
import { SpringTransitionsPage } from './pages/spring-transitions';
import { BUTTON_SPRITE, BUTTON_TEXT_STYLE, COLOR, PANEL_SPRITE, PIXEL_SPRITE } from './theme';

const STAGE_WIDTH = 1920;
const STAGE_HEIGHT = 1080;

/** Content area available to pages, below the page header. */
export const CONTENT_WIDTH = 1504;
export const CONTENT_HEIGHT = 912;

const CONTENT_SCROLLBAR_WIDTH = 20;
const CONTENT_SCROLLBAR_X = CONTENT_WIDTH + 8;
const MIN_CONTENT_SCROLLBAR_HEIGHT = 56;
const CONTENT_SCROLLBAR_BOUNDS: [number, number, number, number] = [0.1, 0.1, 0.1, 0.1];

const SIDEBAR_X = 44;
const SIDEBAR_WIDTH = 292;
const NAV_PANEL_Y = 96;
const NAV_PANEL_HEIGHT = 880;
const NAV_SCROLL_HEIGHT = NAV_PANEL_HEIGHT - 24;

interface GalleryPageDefinition {
  key: string;
  title: string;
  description: string;
  component: ComponentType;
}

const PAGES: readonly GalleryPageDefinition[] = [
  {
    key: 'primitives',
    title: '基础组件',
    description: 'Sprite、Text、Clip、Animation 与通用节点属性。',
    component: PrimitivesPage,
  },
  { key: 'layouts', title: '布局', description: 'VBox、HBox、测量、对齐与动态重排。', component: LayoutsPage },
  {
    key: 'controls',
    title: '封装组件',
    description: 'Button、Checkbox、Select、Slider 与 ScrollView。',
    component: ControlsPage,
  },
  {
    key: 'filters',
    title: 'Filter 滤镜',
    description: '对节点自身子树的离屏渲染结果应用滤镜链。',
    component: FiltersPage,
  },
  {
    key: 'backdrops',
    title: 'Backdrop 背景滤镜',
    description: '处理节点背后已经绘制的画面，之后绘制的内容保持清晰。',
    component: BackdropsPage,
  },
  {
    key: 'spring-transitions',
    title: 'Spring 动画',
    description: 'useSpring 属性动画与 useTransition 进出场动画。',
    component: SpringTransitionsPage,
  },
  {
    key: 'shader-transitions',
    title: 'Shader 转场',
    description: '双通道画面的 GPU 转场状态机。',
    component: ShaderTransitionsPage,
  },
  {
    key: 'raw-shaders',
    title: '自定义 Shader',
    description: 'Raw WGSL、参数槽与时间控制。',
    component: RawShadersPage,
  },
];

function NavItem({
  index,
  title,
  selected,
  onPress,
}: {
  index: number;
  title: string;
  selected: boolean;
  onPress: () => void;
}) {
  return (
    <container>
      <Button
        sprite={{
          ...BUTTON_SPRITE,
          targetWidth: 268,
          targetHeight: 56,
          tint: selected ? COLOR.controlTintActive : COLOR.panelTint,
        }}
        text={`${String(index + 1).padStart(2, '0')}  ${title}`}
        textStyle={{
          ...BUTTON_TEXT_STYLE,
          fillColor: selected ? COLOR.accent : COLOR.navText,
        }}
        textAlign="left"
        textOffsetX={selected ? 30 : 18}
        onPress={onPress}
      />
      {selected ? (
        <sprite src={PIXEL_SPRITE} tint={COLOR.accent} targetWidth={4} targetHeight={28} x={12} y={14} />
      ) : null}
    </container>
  );
}

export function Gallery() {
  const stageSize = getStageSize();
  const scale = Math.min(stageSize.width / STAGE_WIDTH, stageSize.height / STAGE_HEIGHT);
  const [currentPageKey, setCurrentPageKey] = useState<string>('primitives');
  const navigationScroll = useScrollView({ viewportHeight: NAV_SCROLL_HEIGHT });
  const contentScroll = useScrollView({ viewportHeight: CONTENT_HEIGHT });
  const currentPage = PAGES.find((page) => page.key === currentPageKey) ?? PAGES[0];
  const CurrentPage = currentPage.component;
  const { contentHeight, maxScroll, scrollOffset, scrollTo, scrollToRatio } = contentScroll;
  const showContentScrollbar = maxScroll > 0;
  const contentScrollbarHeight = useMemo(() => {
    if (!showContentScrollbar || contentHeight <= 0) return 0;

    return Math.min(
      CONTENT_HEIGHT,
      Math.max(MIN_CONTENT_SCROLLBAR_HEIGHT, (CONTENT_HEIGHT * CONTENT_HEIGHT) / contentHeight),
    );
  }, [contentHeight, showContentScrollbar]);
  const contentScrollbarOffset = useMemo(() => {
    if (!showContentScrollbar || contentScrollbarHeight <= 0) return scrollOffset.to(() => 0);

    return scrollOffset.to((value) => (value / maxScroll) * (CONTENT_HEIGHT - contentScrollbarHeight));
  }, [contentScrollbarHeight, maxScroll, scrollOffset, showContentScrollbar]);
  const draggingContentScrollbarRef = useRef(false);
  const contentDragStartClientYRef = useRef(0);
  const contentDragStartRatioRef = useRef(0);
  const contentScrollbarTravel = CONTENT_HEIGHT - contentScrollbarHeight;

  const handleContentScrollbarDragStart = useCallback(
    (event: MouseEvent | TouchEvent) => {
      if (!showContentScrollbar || contentScrollbarTravel <= 0) return;

      event.stopPropagation();
      draggingContentScrollbarRef.current = true;
      contentDragStartClientYRef.current = event.clientY;
      contentDragStartRatioRef.current = maxScroll <= 0 ? 0 : scrollOffset.get() / maxScroll;
    },
    [contentScrollbarTravel, maxScroll, scrollOffset, showContentScrollbar],
  );

  const handleContentScrollbarDragMove = useCallback(
    (event: MouseEvent | TouchEvent) => {
      if (!draggingContentScrollbarRef.current || contentScrollbarTravel <= 0) return;

      const deltaY = event.clientY - contentDragStartClientYRef.current;
      scrollToRatio(contentDragStartRatioRef.current + deltaY / contentScrollbarTravel, true);
    },
    [contentScrollbarTravel, scrollToRatio],
  );

  const handleContentScrollbarDragEnd = useCallback(() => {
    draggingContentScrollbarRef.current = false;
  }, []);

  useEffect(() => {
    const cleanups = [
      addEventListener('mousemove', handleContentScrollbarDragMove),
      addEventListener('touchmove', handleContentScrollbarDragMove),
      addEventListener('mouseup', handleContentScrollbarDragEnd),
      addEventListener('touchend', handleContentScrollbarDragEnd),
      addEventListener('touchcancel', handleContentScrollbarDragEnd),
    ];

    return () => {
      for (const cleanup of cleanups) {
        cleanup();
      }
    };
  }, [handleContentScrollbarDragEnd, handleContentScrollbarDragMove]);

  return (
    <container label="Moyu Gallery" x={stageSize.width / 2} y={stageSize.height / 2} scale={scale}>
      <sprite src="images/bg.png" pivot={[0.5, 0.5]} />
      <container x={-STAGE_WIDTH / 2} y={-STAGE_HEIGHT / 2}>
        {/* Sidebar: nav panel top aligns with the content area at y=136. */}
        <container x={SIDEBAR_X} y={40}>
          <text text="末语 Gallery" fontSize={46} fillColor={COLOR.pageTitle} />

          <sprite {...PANEL_SPRITE} targetWidth={SIDEBAR_WIDTH} targetHeight={NAV_PANEL_HEIGHT} y={NAV_PANEL_Y}>
            <container x={12} y={12}>
              <ScrollView
                width={SIDEBAR_WIDTH - 24}
                height={NAV_SCROLL_HEIGHT}
                controller={navigationScroll}
                contentProps={{ gap: 8 }}
              >
                {PAGES.map((page, index) => (
                  <NavItem
                    key={page.key}
                    index={index}
                    title={page.title}
                    selected={page.key === currentPage.key}
                    onPress={() => {
                      scrollTo(0, true);
                      setCurrentPageKey(page.key);
                    }}
                  />
                ))}
              </ScrollView>
            </container>
          </sprite>

          <text text="MPL-2.0 · @momoyu-ink/kit" fontSize={16} fillColor={COLOR.dim} y={992} />
        </container>

        {/* Page header: title + description + divider, content starts below. */}
        <container x={372} y={40}>
          <text text={currentPage.title} fontSize={40} fillColor={COLOR.pageTitle} />
          <text text={currentPage.description} fontSize={20} fillColor={COLOR.pageDescription} y={56} />
        </container>

        {/* Page content: 1504x912, ending 32px above the stage bottom. */}
        <container x={372} y={136}>
          <ScrollView width={CONTENT_WIDTH} height={CONTENT_HEIGHT} controller={contentScroll}>
            <CurrentPage />
          </ScrollView>
          {showContentScrollbar ? (
            <animated.sprite
              src="images/chip.png"
              mode="nineslice"
              bounds={CONTENT_SCROLLBAR_BOUNDS}
              targetWidth={CONTENT_SCROLLBAR_WIDTH}
              targetHeight={contentScrollbarHeight}
              x={CONTENT_SCROLLBAR_X}
              y={contentScrollbarOffset}
              cursor="pointer"
              opacity={0.92}
              tint={COLOR.controlTint}
              onMouseDown={handleContentScrollbarDragStart}
              onTouchStart={handleContentScrollbarDragStart}
            />
          ) : null}
        </container>
      </container>
    </container>
  );
}
