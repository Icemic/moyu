import {
  Button,
  Checkbox,
  ScrollView,
  Select,
  Slider,
  useScrollView,
  type SpineCommand,
  type SpineEvent,
  type SpineMetadata,
  type Node,
} from '@momoyu-ink/kit';
import { useLingui } from '@lingui/react/macro';
import { useCallback, useRef, useState, type ReactNode } from 'react';
import { Panel, SectionTabs } from '../components/chrome';
import {
  BUTTON_SPRITE,
  BUTTON_TEXT_STYLE,
  CHECKBOX_CHECKED_SPRITE,
  CHECKBOX_UNCHECKED_SPRITE,
  COLOR,
  SELECT_LIST,
  SELECT_OPTION,
  SELECT_TRIGGER,
  SLIDER_THUMB,
  SLIDER_TRACK,
  TEXT,
} from '../theme';

/**
 * The page fits the gallery content area (1504x912). The preview panel is a
 * stable sibling of the tab areas: putting it inside a tab would unmount the
 * `spine` node and drop the model on every switch.
 *
 *   left  744  model bar, tabs and the current tab body
 *   right 744  preview and the event dock
 */
const COLUMN_WIDTH = 744;
const COLUMN_GAP = 16;
const CONTENT_HEIGHT = 912;

const MODEL_BAR_HEIGHT = 174;
const MAIN_TAB_GAP = 12;
const MAIN_TABS_HEIGHT = 48;
const MAIN_TAB_HEIGHT =
  CONTENT_HEIGHT - MODEL_BAR_HEIGHT - COLUMN_GAP - MAIN_TABS_HEIGHT - MAIN_TAB_GAP;
/** Usable height inside a tab panel: panel height minus the content inset. */
const TAB_BODY_HEIGHT = MAIN_TAB_HEIGHT - 40;

const PREVIEW_HEIGHT = 730;
const VIEW_WIDTH = COLUMN_WIDTH - 40;
const VIEW_HEIGHT = 580;
const DOCK_HEIGHT = CONTENT_HEIGHT - PREVIEW_HEIGHT - COLUMN_GAP;

const SCROLL_WIDTH = COLUMN_WIDTH - 40;
const LOG_LIMIT = 24;
const LOG_PAGE_SIZE = 6;
const ANIMATION_PAGE_SIZE = 9;
const BUTTONS_PER_ROW = 3;

/** Range the time scale slider covers. */
const MIN_TIME_SCALE = 0;
const MAX_TIME_SCALE = 3;

const GROUP_LABEL = { fontSize: 18, fillColor: COLOR.dim } as const;
const SMALL_BUTTON = { ...BUTTON_SPRITE, targetWidth: 200, targetHeight: 48 };
const WIDE_BUTTON = { ...BUTTON_SPRITE, targetWidth: 300, targetHeight: 48 };
const PAGE_BUTTON = { ...BUTTON_SPRITE, targetWidth: 110, targetHeight: 48 };

/**
 * Models the page loads from `assets/spine/`. The samples and the plugin binary
 * are not part of the repository; the diagnostics tab says where they go.
 *
 * The sample exports name their atlas after the model rather than after the
 * skeleton file (`spineboy-pro.json` sits next to `spineboy.atlas`), so every
 * entry spells the atlas out instead of relying on the derived default.
 */
const MODELS = [
  {
    value: 'spine/spineboy/spineboy-pro.json',
    atlas: 'spine/spineboy/spineboy.atlas',
    label: 'spineboy (json)',
  },
  {
    value: 'spine/spineboy/spineboy-pro.skel',
    atlas: 'spine/spineboy/spineboy.atlas',
    label: 'spineboy (skel)',
  },
  {
    value: 'spine/spineboy/spineboy-ess.json',
    atlas: 'spine/spineboy/spineboy.atlas',
    label: 'spineboy-ess (json)',
  },
  {
    value: 'spine/spineboy/spineboy-ess.skel',
    atlas: 'spine/spineboy/spineboy.atlas',
    label: 'spineboy-ess (skel)',
  },
  {
    value: 'spine/spineboy/spineboy-pro.json',
    atlas: 'spine/spineboy/spineboy-pma.atlas',
    label: 'spineboy (pma atlas)',
  },
  {
    value: 'spine/alien/alien-pro.json',
    atlas: 'spine/alien/alien.atlas',
    label: 'alien',
  },
  { value: 'spine/owl/owl-pro.json', atlas: 'spine/owl/owl.atlas', label: 'owl' },
  {
    value: 'spine/dragon/dragon-ess.json',
    atlas: 'spine/dragon/dragon.atlas',
    label: 'dragon (6 pages)',
  },
  {
    value: 'spine/celestial-circus/celestial-circus-pro.json',
    atlas: 'spine/celestial-circus/celestial-circus.atlas',
    label: 'celestial-circus (clipping)',
  },
] as const;

const PHYSICS_MODES = [
  { value: 'none', label: 'None' },
  { value: 'pose', label: 'Pose' },
  { value: 'update', label: 'Update' },
] as const;

type PhysicsMode = (typeof PHYSICS_MODES)[number]['value'];
type MainTab = 'animate' | 'settings' | 'diagnose';

function chunk<T>(items: readonly T[], size: number): T[][] {
  const rows: T[][] = [];
  for (let index = 0; index < items.length; index += size) {
    rows.push(items.slice(index, index + size));
  }

  return rows;
}

/** Largest scale that keeps the whole reference rectangle inside the view. */
function fitScale(metadata: SpineMetadata | null): number {
  if (!metadata) return 1;
  const { width, height } = metadata.bounds;
  if (width <= 0 || height <= 0) return 1;

  return Math.min(VIEW_WIDTH / width, VIEW_HEIGHT / height) * 0.94;
}

function ItemRows({
  items,
  render,
  perRow = BUTTONS_PER_ROW,
  gap = 10,
}: {
  items: readonly string[];
  render: (item: string) => ReactNode;
  perRow?: number;
  gap?: number;
}) {
  return (
    <vbox gap={gap}>
      {chunk(items, perRow).map((row, index) => (
        <hbox key={index} gap={gap}>
          {row.map((item) => render(item))}
        </hbox>
      ))}
    </vbox>
  );
}

/** Paging controls on their own row; renders nothing for a single page. */
function Pager({
  page,
  pages,
  onChange,
}: {
  page: number;
  pages: number;
  onChange: (page: number) => void;
}) {
  if (pages <= 1) return null;

  return (
    <hbox gap={12} alignItems="center">
      <Button
        sprite={PAGE_BUTTON}
        text="<<"
        textStyle={BUTTON_TEXT_STYLE}
        disabled={page <= 0}
        onPress={() => onChange(page - 1)}
      />
      <text {...TEXT.body} text={`${page + 1} / ${pages}`} fillColor={COLOR.accent} />
      <Button
        sprite={PAGE_BUTTON}
        text=">>"
        textStyle={BUTTON_TEXT_STYLE}
        disabled={page >= pages - 1}
        onPress={() => onChange(page + 1)}
      />
    </hbox>
  );
}

/** One labelled read-only value in the diagnostics tab. */
function InfoRow({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <hbox gap={12} alignItems="start">
      <vbox width={150}>
        <text {...GROUP_LABEL} text={label} />
      </vbox>
      <vbox width={SCROLL_WIDTH - 180}>
        <text {...TEXT.body} text={value} fillColor={accent ? COLOR.accent : COLOR.text} />
      </vbox>
    </hbox>
  );
}

export function SpinePage() {
  const { t } = useLingui();
  const spineRef = useRef<Node>(null);

  const [src, setSrc] = useState<string>(MODELS[0].value);
  const [metadata, setMetadata] = useState<SpineMetadata | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [log, setLog] = useState<string[]>([]);

  const [skin, setSkin] = useState('');
  const [paused, setPaused] = useState(false);
  const [timeScale, setTimeScale] = useState(1);
  const [physics, setPhysics] = useState<PhysicsMode>('update');
  const [hitArea, setHitArea] = useState(true);
  const [hitResult, setHitResult] = useState<string | null>(null);

  const [mainTab, setMainTab] = useState<MainTab>('animate');
  const [animationPage, setAnimationPage] = useState(0);
  const [logPage, setLogPage] = useState(0);

  const animateScroll = useScrollView({ viewportHeight: TAB_BODY_HEIGHT });
  const settingsScroll = useScrollView({ viewportHeight: TAB_BODY_HEIGHT });
  const diagnoseScroll = useScrollView({ viewportHeight: TAB_BODY_HEIGHT });

  const pushLog = useCallback((text: string) => {
    setLog((entries) => [text, ...entries].slice(0, LOG_LIMIT));
  }, []);

  const run = useCallback(
    (command: SpineCommand, description: string) => {
      spineRef.current?.executeCommand(command);
      pushLog(description);
    },
    [pushLog],
  );

  const animations = metadata?.animations ?? [];
  const animationPages = Math.max(1, Math.ceil(animations.length / ANIMATION_PAGE_SIZE));
  const animationSlice = animations.slice(
    animationPage * ANIMATION_PAGE_SIZE,
    (animationPage + 1) * ANIMATION_PAGE_SIZE,
  );
  const logPages = Math.max(1, Math.ceil(log.length / LOG_PAGE_SIZE));
  const logSlice = log.slice(logPage * LOG_PAGE_SIZE, (logPage + 1) * LOG_PAGE_SIZE);

  const skins = metadata?.skins ?? [];
  const bounds = metadata?.bounds;
  const scale = fitScale(metadata);
  const modelWidth = (bounds?.width ?? 0) * scale;
  const modelHeight = (bounds?.height ?? 0) * scale;
  // The node's own coordinates start at its top-left corner, so the model is
  // centered by placing the node there.
  const modelX = (VIEW_WIDTH - modelWidth) / 2;
  const modelY = (VIEW_HEIGHT - modelHeight) / 2;

  const handleReady = useCallback(
    (event: Extract<SpineEvent, { type: 'ready' }>) => {
      setMetadata(event);
      setError(null);
      setSkin(event.skins[0] ?? '');
      setAnimationPage(0);
      setLogPage(0);
      pushLog(
        t`ready: ${event.dataVersion} · ${event.animations.length} animations · ` +
          `${event.skins.length} skins · ${event.pages.length} pages`,
      );
    },
    [pushLog, t],
  );

  const handleError = useCallback(
    (event: Extract<SpineEvent, { type: 'error' }>) => {
      setError(event.message);
      pushLog(t`error: ${event.message}`);
    },
    [pushLog, t],
  );

  const handleHit = useCallback(
    (event: Extract<SpineEvent, { type: 'hit' }>) => {
      setHitResult(event.name);
      pushLog(t`hit: ${event.name}`);
    },
    [pushLog, t],
  );

  const handleTrackEvent = useCallback(
    (event: Extract<SpineEvent, { type: 'trackEvent' }>) => {
      pushLog(t`event ${event.name} on track ${event.track}`);
    },
    [pushLog, t],
  );

  const handleTrackDispose = useCallback(
    (event: Extract<SpineEvent, { type: 'trackDispose' }>) => {
      pushLog(
        t`dispose ${event.animation || '-'} (entry ${event.id.entry} · epoch ${event.id.epoch})`,
      );
    },
    [pushLog, t],
  );

  /// Switches the loaded model. The node reloads on the `src` change and reports
  /// a fresh `ready`, which resets the per-model state; nothing has to be cleared
  /// here, and the old model's tracks die with it.
  const handleModelChange = useCallback((value: string) => {
    setSrc(value);
    setMetadata(null);
    setError(null);
    setHitResult(null);
  }, []);

  const current = MODELS.find((model) => model.value === src) ?? MODELS[0];

  return (
    <hbox gap={COLUMN_GAP} alignItems="start">
      <vbox gap={COLUMN_GAP} width={COLUMN_WIDTH} height={CONTENT_HEIGHT}>
        <Panel title={t`模型`} width={COLUMN_WIDTH} height={MODEL_BAR_HEIGHT} zIndex={2}>
          <vbox gap={8} zIndex={2}>
            <hbox gap={16} alignItems="center">
              <Select
                value={src}
                onValueChange={handleModelChange}
                options={MODELS.map((item) => ({ text: item.label, value: item.value }))}
                trigger={{ ...SELECT_TRIGGER, targetWidth: 300 }}
                list={{ ...SELECT_LIST, targetWidth: 300 }}
                option={{ ...SELECT_OPTION, targetWidth: 294 }}
                textStyle={BUTTON_TEXT_STYLE}
              />
              <text
                {...TEXT.body}
                text={
                  bounds
                    ? `${bounds.width.toFixed(0)} × ${bounds.height.toFixed(0)}`
                    : t`未加载`
                }
                fillColor={metadata ? COLOR.accent : COLOR.dim}
              />
            </hbox>
            <text
              {...TEXT.caption}
              boxWidth={COLUMN_WIDTH - 60}
              lineHeight={1.4}
              fillColor={error ? COLOR.accent : COLOR.dim}
              text={
                error
                  ? error.length > 96
                    ? `${error.slice(0, 96)}…`
                    : error
                  : metadata
                    ? t`模型已就绪；参考包围盒即布局尺寸，动画不会改变它。`
                    : t`等待 ready 事件；无响应时到「接入诊断」查看错误与放置说明。`
              }
            />
          </vbox>
        </Panel>

        <SectionTabs
          value={mainTab}
          onChange={setMainTab}
          tabWidth={244}
          options={[
            { value: 'animate', label: t`动画与事件` },
            { value: 'settings', label: t`外观与播放` },
            { value: 'diagnose', label: t`接入诊断` },
          ]}
        />

        <Panel width={COLUMN_WIDTH} height={MAIN_TAB_HEIGHT}>
          {mainTab === 'animate' ? (
            <ScrollView
              width={SCROLL_WIDTH}
              height={TAB_BODY_HEIGHT}
              controller={animateScroll}
              contentProps={{ gap: 12 }}
            >
              <text {...GROUP_LABEL} text={t`动画`} />
              {animations.length === 0 ? (
                <text {...TEXT.caption} text={t`加载模型后列出动画。`} />
              ) : (
                <vbox gap={12}>
                  <ItemRows
                    items={animationSlice.map((animation) => animation.name)}
                    render={(name) => (
                      <Button
                        key={name}
                        sprite={SMALL_BUTTON}
                        text={name}
                        textStyle={BUTTON_TEXT_STYLE}
                        onPress={() =>
                          run(
                            { subCommand: 'setAnimation', track: 0, animation: name, loop: true },
                            `setAnimation(${name})`,
                          )
                        }
                      />
                    )}
                  />
                  <Pager page={animationPage} pages={animationPages} onChange={setAnimationPage} />
                  <text {...TEXT.caption} text={t`点击即在本轨替换；需要排队时用下方按钮。`} />
                </vbox>
              )}

              <text {...GROUP_LABEL} text={t`轨道`} />
              <hbox gap={10}>
                <Button
                  sprite={WIDE_BUTTON}
                  text={t`上层叠加当前动画`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() => {
                    const name = animations[0]?.name;
                    if (!name) return;
                    run(
                      {
                        subCommand: 'addAnimation',
                        track: 1,
                        animation: name,
                        loop: false,
                        mix: 0.2,
                      },
                      `addAnimation(${name}, track 1)`,
                    );
                  }}
                />
                <Button
                  sprite={WIDE_BUTTON}
                  text={t`淡出轨道 0`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() => run({ subCommand: 'clearTrack', track: 0, mix: 0.3 }, 'clearTrack(0, 0.3)')}
                />
              </hbox>
              <hbox gap={10}>
                <Button
                  sprite={WIDE_BUTTON}
                  text={t`清空全部轨道`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() => run({ subCommand: 'clearTracks' }, 'clearTracks()')}
                />
                <Button
                  sprite={WIDE_BUTTON}
                  text={t`复位到 setup pose`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() => run({ subCommand: 'resetToSetupPose' }, 'resetToSetupPose()')}
                />
              </hbox>

              <text {...GROUP_LABEL} text={t`过渡`} />
              <hbox gap={10}>
                <Button
                  sprite={WIDE_BUTTON}
                  text={t`默认混合 0.2s`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() => run({ subCommand: 'setDefaultMix', duration: 0.2 }, 'setDefaultMix(0.2)')}
                />
                <Button
                  sprite={WIDE_BUTTON}
                  text={t`默认混合 0s`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() => run({ subCommand: 'setDefaultMix', duration: 0 }, 'setDefaultMix(0)')}
                />
              </hbox>

              <text {...GROUP_LABEL} text={t`命中`} />
              <hbox gap={10} alignItems="center">
                <Button
                  sprite={WIDE_BUTTON}
                  text={t`启用 spineboy 命中区`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() =>
                    run(
                      { subCommand: 'setAttachment', slot: 'head-bb', name: 'head' },
                      'setAttachment(head-bb, head)',
                    )
                  }
                />
                <Button
                  sprite={WIDE_BUTTON}
                  text={t`点击模型测试命中`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() =>
                    run(
                      { subCommand: 'hitTest', x: modelWidth / 2, y: modelHeight / 2 },
                      'hitTest(center)',
                    )
                  }
                />
                <text {...TEXT.body} text={hitResult ?? t`（无）`} fillColor={COLOR.accent} />
              </hbox>
              <text
                {...TEXT.caption}
                boxWidth={SCROLL_WIDTH}
                lineHeight={1.5}
                text={t`命中只统计当前启用的包围盒：多数模型默认不启用任何命中区，需要先用 setAttachment 打开某个槽位（spineboy 为 head-bb / head）。`}
              />
            </ScrollView>
          ) : mainTab === 'settings' ? (
            <ScrollView
              width={SCROLL_WIDTH}
              height={TAB_BODY_HEIGHT}
              controller={settingsScroll}
              contentProps={{ gap: 12 }}
            >
              <text {...GROUP_LABEL} text={t`播放`} />
              <hbox gap={16} alignItems="center">
                <Checkbox
                  checked={paused}
                  onCheckedChange={setPaused}
                  uncheckedSprite={CHECKBOX_UNCHECKED_SPRITE}
                  checkedSprite={CHECKBOX_CHECKED_SPRITE}
                />
                <text {...TEXT.body} text={t`暂停`} />
                <text {...TEXT.caption} text={t`暂停时求值照常进行，切换皮肤与附件立即可见。`} />
              </hbox>

              <hbox gap={16} alignItems="center">
                <text {...TEXT.body} text={t`时间缩放`} />
                <Slider
                  value={(timeScale - MIN_TIME_SCALE) / (MAX_TIME_SCALE - MIN_TIME_SCALE)}
                  onValueChange={(value) =>
                    setTimeScale(MIN_TIME_SCALE + value * (MAX_TIME_SCALE - MIN_TIME_SCALE))
                  }
                  track={SLIDER_TRACK}
                  thumb={SLIDER_THUMB}
                />
                <text {...TEXT.body} text={timeScale.toFixed(2)} fillColor={COLOR.accent} />
              </hbox>

              <hbox gap={16} alignItems="center">
                <text {...TEXT.body} text={t`物理`} />
                <Select
                  value={physics}
                  onValueChange={(value) => setPhysics(value as PhysicsMode)}
                  options={PHYSICS_MODES.map((mode) => ({ text: mode.label, value: mode.value }))}
                  trigger={{ ...SELECT_TRIGGER, targetWidth: 240 }}
                  list={{ ...SELECT_LIST, targetWidth: 240 }}
                  option={{ ...SELECT_OPTION, targetWidth: 234 }}
                  textStyle={BUTTON_TEXT_STYLE}
                />
                <Button
                  sprite={PAGE_BUTTON}
                  text={t`重置物理`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() => run({ subCommand: 'resetPhysics' }, 'resetPhysics()')}
                />
              </hbox>

              <text {...GROUP_LABEL} text={t`皮肤`} />
              {skins.length === 0 ? (
                <text {...TEXT.caption} text={t`该模型没有命名皮肤，或尚未就绪。`} />
              ) : (
                <Select
                  value={skin}
                  onValueChange={(value) => {
                    setSkin(value);
                    pushLog(`skin(${value || 'default'})`);
                  }}
                  options={skins.map((name) => ({ text: name || t`（默认）`, value: name }))}
                  trigger={{ ...SELECT_TRIGGER, targetWidth: 360 }}
                  list={{ ...SELECT_LIST, targetWidth: 360 }}
                  option={{ ...SELECT_OPTION, targetWidth: 354 }}
                  textStyle={BUTTON_TEXT_STYLE}
                />
              )}

              <text {...GROUP_LABEL} text={t`指针`} />
              <hbox gap={16} alignItems="center">
                <Checkbox
                  checked={hitArea}
                  onCheckedChange={setHitArea}
                  uncheckedSprite={CHECKBOX_UNCHECKED_SPRITE}
                  checkedSprite={CHECKBOX_CHECKED_SPRITE}
                />
                <text {...TEXT.body} text={t`hitArea 属性（按下即上报命中）`} />
              </hbox>
            </ScrollView>
          ) : (
            <ScrollView
              width={SCROLL_WIDTH}
              height={TAB_BODY_HEIGHT}
              controller={diagnoseScroll}
              contentProps={{ gap: 10 }}
            >
              <InfoRow label={t`骨架`} value={src} />
              <InfoRow label={t`数据版本`} value={metadata?.dataVersion || t`（无）`} accent />
              <InfoRow
                label={t`参考包围盒`}
                value={
                  bounds
                    ? `${bounds.x.toFixed(1)}, ${bounds.y.toFixed(1)}, ` +
                      `${bounds.width.toFixed(1)} × ${bounds.height.toFixed(1)}`
                    : t`（无）`
                }
              />
              <InfoRow label={t`皮肤`} value={skins.join('、') || t`（无）`} />
              <InfoRow
                label={t`包围盒`}
                value={(metadata?.boundingBoxes ?? []).join('、') || t`（无）`}
              />
              <InfoRow
                label={t`页`}
                value={
                  (metadata?.pages ?? [])
                    .map((page) => `${page.width}×${page.height}${page.premultiplied ? ' pma' : ''}`)
                    .join('、') || t`（无）`
                }
              />
              <InfoRow label={t`当前错误`} value={error ?? t`（无）`} accent={error !== null} />

              <text {...GROUP_LABEL} text={t`事件`} />
              {logSlice.length === 0 ? (
                <text {...TEXT.caption} text={t`尚无事件。`} />
              ) : (
                logSlice.map((entry, index) => (
                  <text key={`${index}-${entry}`} {...TEXT.caption} text={entry} />
                ))
              )}
              <Pager page={logPage} pages={logPages} onChange={setLogPage} />

              <text {...GROUP_LABEL} text={t`放置说明`} />
              <text
                {...TEXT.caption}
                boxWidth={SCROLL_WIDTH}
                lineHeight={1.5}
                text={t`把骨架与图集放到 assets/spine/（如 spineboy.json、spineboy.atlas 与页图），把插件二进制放到引擎可执行文件旁或设置 MOYU_SPINE_PLUGIN。素材与插件都不进仓库。`}
              />
            </ScrollView>
          )}
        </Panel>
      </vbox>

      <vbox gap={COLUMN_GAP} width={COLUMN_WIDTH} height={CONTENT_HEIGHT}>
        <Panel title={t`预览`} width={COLUMN_WIDTH} height={PREVIEW_HEIGHT}>
          <clip width={VIEW_WIDTH} height={VIEW_HEIGHT}>
            <container>
              <spine
                ref={spineRef}
                src={current.value}
                atlas={current.atlas}
                skin={skin}
                paused={paused}
                timeScale={timeScale}
                physics={physics}
                hitArea={hitArea}
                x={modelX}
                y={modelY}
                scaleX={scale}
                scaleY={scale}
                onReady={handleReady}
                onError={handleError}
                onHit={handleHit}
                onTrackEvent={handleTrackEvent}
                onTrackDispose={handleTrackDispose}
              />
            </container>
          </clip>
        </Panel>

        <Panel title={t`最新事件`} width={COLUMN_WIDTH} height={DOCK_HEIGHT}>
          <vbox gap={6}>
            {log.slice(0, 3).map((entry, index) => (
              <text key={`${index}-${entry}`} {...TEXT.caption} text={entry} />
            ))}
          </vbox>
        </Panel>
      </vbox>
    </hbox>
  );
}
