import { useLingui } from '@lingui/react/macro';
import {
  addEventListener,
  animated,
  Button,
  getStageSize,
  type MouseEvent,
  ScrollView,
  Select,
  type TouchEvent,
  useScrollView,
} from '@momoyu-ink/kit';
import { type ComponentType, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { activateLocale, DEFAULT_LOCALE, type Locale } from './i18n';
import { BackdropsPage } from './pages/backdrops';
import { ControlsPage } from './pages/controls';
import { CursorPage } from './pages/cursor';
import { FiltersPage } from './pages/filters';
import { InputPage } from './pages/input';
import { LayoutsPage } from './pages/layouts';
import { Live2dPage } from './pages/live2d';
import { PrimitivesPage } from './pages/primitives';
import { RawShadersPage } from './pages/raw-shaders';
import { ShaderTransitionsPage } from './pages/shader-transitions';
import { SpinePage } from './pages/spine';
import { SpringTransitionsPage } from './pages/spring-transitions';
import { SteamPage } from './pages/steam';
import {
  applyGalleryMode,
  BUTTON_SELECTED_SPRITE,
  BUTTON_SPRITE,
  BUTTON_TEXT_STYLE,
  backgroundSprite,
  COLOR,
  currentGalleryMode,
  type GalleryMode,
  PIXEL_SPRITE,
  SELECT_LIST,
  SELECT_OPTION,
  SELECT_TRIGGER,
} from './theme';

const STAGE_WIDTH = 1920;
const STAGE_HEIGHT = 1080;

/** Content area available to pages, below the page header. */
export const CONTENT_WIDTH = 1504;
export const CONTENT_HEIGHT = 912;

const CONTENT_SCROLLBAR_WIDTH = 15;
const CONTENT_SCROLLBAR_X = CONTENT_WIDTH + 8;
const MIN_CONTENT_SCROLLBAR_HEIGHT = 56;
const CONTENT_SCROLLBAR_BOUNDS: [number, number, number, number] = [0.1, 0.1, 0.1, 0.1];

/**
 * Sidebar column, nav list and content share one gutter of `SIDEBAR_PAD` on the left and
 * right: the grid background starts where the column ends, so the content column keeps
 * the same padding on both sides (44px).
 */
const SIDEBAR_PAD = 28;
const SIDEBAR_WIDTH = 272;
const SIDEBAR_X = SIDEBAR_PAD;
/** Content starts at 372 and ends at 1876, so the column stops 44px before it. */
const SIDEBAR_COLUMN_WIDTH = 328;
/** Locale and appearance controls share the row under the brand mark. */
const LOCALE_SELECT_WIDTH = 152;
const APPEARANCE_BUTTON_WIDTH = SIDEBAR_WIDTH - LOCALE_SELECT_WIDTH - 12;
const NAV_PANEL_Y = 164;
const NAV_PANEL_HEIGHT = 812;
const NAV_SCROLL_HEIGHT = NAV_PANEL_HEIGHT - 24;

interface GalleryPageDefinition {
  key: string;
  title: string;
  description: string;
  component: ComponentType;
}

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
          ...(selected ? BUTTON_SELECTED_SPRITE : BUTTON_SPRITE),
          targetWidth: SIDEBAR_WIDTH,
          targetHeight: 56,
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
    </container>
  );
}

export function Gallery() {
  const { t } = useLingui();
  const stageSize = getStageSize();
  const scale = Math.min(stageSize.width / STAGE_WIDTH, stageSize.height / STAGE_HEIGHT);
  const [locale, setLocale] = useState<Locale>(DEFAULT_LOCALE);
  const [mode, setMode] = useState<GalleryMode>(currentGalleryMode);
  const [currentPageKey, setCurrentPageKey] = useState<string>('primitives');
  const pages: readonly GalleryPageDefinition[] = [
    {
      key: 'primitives',
      title: t`基础组件`,
      description: t`Sprite、Text、Clip、Animation 与通用节点属性。`,
      component: PrimitivesPage,
    },
    { key: 'layouts', title: t`布局`, description: t`VBox、HBox、测量、对齐与动态重排。`, component: LayoutsPage },
    {
      key: 'controls',
      title: t`封装组件`,
      description: t`Button、Checkbox、Radio、Select、Slider 与 ScrollView。`,
      component: ControlsPage,
    },
    {
      key: 'input',
      title: t`Input 输入框`,
      description: t`Editable 节点、末尾输入、IME、焦点与受控状态。`,
      component: InputPage,
    },
    {
      key: 'cursor',
      title: t`自定义鼠标指针`,
      description: t`cursor 配置、状态查表、default 回退、热点与运行期开启切换。`,
      component: CursorPage,
    },
    {
      key: 'filters',
      title: t`Filter 滤镜`,
      description: t`对节点自身子树的离屏渲染结果应用滤镜链。`,
      component: FiltersPage,
    },
    {
      key: 'backdrops',
      title: t`Backdrop 背景滤镜`,
      description: t`处理节点背后已经绘制的画面，之后绘制的内容保持清晰。`,
      component: BackdropsPage,
    },
    {
      key: 'spring-transitions',
      title: t`Spring 动画`,
      description: t`useSpring 属性动画与 useTransition 进出场动画。`,
      component: SpringTransitionsPage,
    },
    {
      key: 'shader-transitions',
      title: t`Shader 转场`,
      description: t`双通道画面的 GPU 转场状态机。`,
      component: ShaderTransitionsPage,
    },
    {
      key: 'raw-shaders',
      title: t`自定义 Shader`,
      description: t`Raw WGSL、参数槽与时间控制。`,
      component: RawShadersPage,
    },
    {
      key: 'live2d',
      title: t`Live2D（仅桌面端）`,
      description: t`Live2D 模型、动作、表情、参数与遮罩渲染。`,
      component: Live2dPage,
    },
    {
      key: 'spine',
      title: t`Spine（仅桌面端）`,
      description: t`Spine 骨骼动画、多轨播放、双色染色与混合模式。`,
      component: SpinePage,
    },
    {
      key: 'steam',
      title: t`Steam（仅桌面端）`,
      description: t`Achievement 与 Steamworks API 手动测试。`,
      component: SteamPage,
    },
  ];
  const navigationScroll = useScrollView({ viewportHeight: NAV_SCROLL_HEIGHT });
  const contentScroll = useScrollView({ viewportHeight: CONTENT_HEIGHT });
  const currentPage = pages.find((page) => page.key === currentPageKey) ?? pages[0];
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
      <sprite src={backgroundSprite(mode)} pivot={[0.5, 0.5]} />
      <container x={-STAGE_WIDTH / 2} y={-STAGE_HEIGHT / 2}>
        {/* Sidebar: a full-height surface column holding the brand, the two selectors and
            the scrollable navigation list. */}
        <sprite
          src={PIXEL_SPRITE}
          mode="nineslice"
          bounds={[0, 0, 0, 0]}
          tint={COLOR.surface}
          targetWidth={SIDEBAR_COLUMN_WIDTH}
          targetHeight={STAGE_HEIGHT}
        />
        <container x={SIDEBAR_X} y={40}>
          <text text={t`末语 Gallery`} fontSize={46} fontWeight={600} fillColor={COLOR.pageTitle} />

          <Select
            zIndex={2}
            y={96}
            value={locale}
            options={[
              { value: 'zh', text: '中文' },
              { value: 'en', text: 'English' },
              { value: 'ja', text: '日本語' },
            ]}
            trigger={{ ...SELECT_TRIGGER, targetWidth: LOCALE_SELECT_WIDTH }}
            list={{ ...SELECT_LIST, targetWidth: LOCALE_SELECT_WIDTH }}
            option={{ ...SELECT_OPTION, targetWidth: LOCALE_SELECT_WIDTH - 8 }}
            textStyle={BUTTON_TEXT_STYLE}
            textAlign="left"
            textOffsetX={18}
            onValueChange={(value) => {
              const nextLocale = value as Locale;
              activateLocale(nextLocale);
              setLocale(nextLocale);
            }}
          />

          <Button
            x={LOCALE_SELECT_WIDTH + 12}
            y={96}
            sprite={{ ...BUTTON_SPRITE, targetWidth: APPEARANCE_BUTTON_WIDTH, targetHeight: 56 }}
            text={mode === 'light' ? t`浅色` : t`深色`}
            textStyle={BUTTON_TEXT_STYLE}
            onPress={() => {
              const next = mode === 'light' ? 'dark' : 'light';
              applyGalleryMode(next);
              setMode(next);
            }}
          />

          <container y={NAV_PANEL_Y}>
            <ScrollView
              width={SIDEBAR_WIDTH}
              height={NAV_SCROLL_HEIGHT}
              controller={navigationScroll}
              contentProps={{ gap: 8 }}
            >
              {pages.map((page, index) => (
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

          <text text="MPL-2.0 · @momoyu-ink/kit" fontSize={16} fillColor={COLOR.dim} y={992} />
        </container>

        {/* Page header: title + description + divider, content starts below. */}
        <container x={372} y={40}>
          <text text={currentPage.title} fontSize={40} fontWeight={600} fillColor={COLOR.pageTitle} />
          <text text={currentPage.description} fontSize={20} fillColor={COLOR.pageDescription} y={56} />
        </container>

        {/* Page content: 1504x912, ending 32px above the stage bottom. */}
        <container x={372} y={136}>
          <ScrollView width={CONTENT_WIDTH} height={CONTENT_HEIGHT} controller={contentScroll}>
            <CurrentPage key={mode} />
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
