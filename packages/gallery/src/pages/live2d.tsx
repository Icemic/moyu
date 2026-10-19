import {
  addEventListener,
  Button,
  Checkbox,
  ScrollView,
  Select,
  Slider,
  useScrollView,
  type Live2dCommand,
  type Live2dEvent,
  type Live2dMetadata,
  type MouseEvent as MoyuMouseEvent,
  type Node,
  type TouchEvent as MoyuTouchEvent,
  type WheelEvent as MoyuWheelEvent,
} from '@momoyu-ink/kit';
import { useLingui } from '@lingui/react/macro';
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
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
 * The page fits the gallery content area (1504x912) exactly, so the shared
 * scroll view never moves and the preview stays visible. The preview panel is a
 * stable sibling of the tab areas: putting it inside a tab would unmount the
 * `live2d` node and make the plugin drop the model on every switch.
 *
 *   left  744x174  model bar (always visible)
 *   left  744x48   main tabs
 *   left  744x662  current main tab, scrolls internally
 *   right 744x730  preview (always visible)
 *   right 744x166  click feedback dock
 */
const COLUMN_WIDTH = 744;
const COLUMN_GAP = 16;
const CONTENT_HEIGHT = 912;

const MODEL_BAR_HEIGHT = 174;
const MAIN_TABS_HEIGHT = 48;
const MAIN_TAB_GAP = 12;
const MAIN_TAB_HEIGHT =
  CONTENT_HEIGHT - MODEL_BAR_HEIGHT - COLUMN_GAP - MAIN_TABS_HEIGHT - MAIN_TAB_GAP;
/** Usable height inside a tab panel: panel height minus the content inset. */
const TAB_BODY_HEIGHT = MAIN_TAB_HEIGHT - 40;

const PREVIEW_HEIGHT = 730;
/** Live2d view inside the preview panel, below the title. */
const VIEW_WIDTH = COLUMN_WIDTH - 40;
const VIEW_HEIGHT = 580;
/** Top edge of the model in the fitted view, in stage units. */
const VIEW_TOP = 12;
const DOCK_HEIGHT = CONTENT_HEIGHT - PREVIEW_HEIGHT - COLUMN_GAP;

/** Zoom range relative to the fit scale, as a multiplier of it. */
const MIN_ZOOM = 0.5;
const MAX_ZOOM = 6;
/** Zoom change per wheel step; one wheel line is one step. */
const ZOOM_PER_STEP = 0.1;
/** Pointer travel that turns a press into a drag instead of a click. */
const DRAG_THRESHOLD = 6;
const RESET_BUTTON_WIDTH = 132;

const SCROLL_WIDTH = COLUMN_WIDTH - 40;
const LOG_LIMIT = 24;
/** Log entries per page in the diagnostics tab. */
const LOG_PAGE_SIZE = 6;
/** Buttons per row; the layout elements have no wrapping. */
const BUTTONS_PER_ROW = 4;
/** Motion buttons per page, four per row over two rows. */
const MOTION_PAGE_SIZE = 8;
/** Expression buttons per page, three per row over two rows. */
const EXPRESSION_PAGE_SIZE = 6;

const GROUP_LABEL = { fontSize: 18, fillColor: COLOR.dim } as const;
const SMALL_BUTTON = { ...BUTTON_SPRITE, targetWidth: 150, targetHeight: 48 };
const WIDE_BUTTON = { ...BUTTON_SPRITE, targetWidth: 240, targetHeight: 48 };
const EXPRESSION_BUTTON = { ...BUTTON_SPRITE, targetWidth: 226, targetHeight: 48 };
const PAGE_BUTTON = { ...BUTTON_SPRITE, targetWidth: 110, targetHeight: 48 };

/**
 * Models the page loads from `assets/live2d/`. The models and the plugin binary
 * are not part of the repository; the diagnostics tab says where they go.
 */
const MODELS = [
  { value: 'live2d/Hiyori/Hiyori.model3.json', label: 'Hiyori' },
  { value: 'live2d/Hiyori2/Hiyori.model3.json', label: 'Hiyori2' },
  { value: 'live2d/Mao/Mao.model3.json', label: 'Mao' },
  { value: 'live2d/Haru/Haru.model3.json', label: 'Haru' },
] as const;

/**
 * Part ids of the sample models. The `ready` metadata reports parameters and hit
 * areas but not parts, so the page keeps a small list per model to exercise
 * `setPartOpacity`.
 */
const PART_IDS: Record<string, readonly string[]> = {
  Hiyori: ['PartHairFront', 'PartHairSide', 'PartHairBack', 'PartFace', 'PartEye', 'PartMouth'],
  Hiyori2: ['PartHairFront', 'PartHairSide', 'PartHairBack', 'PartFace', 'PartEye', 'PartMouth'],
  Mao: ['PartHairFront', 'PartHairSide', 'PartHat', 'PartFace', 'PartRabbit', 'PartEffect'],
  Haru: ['Part01HairFront001', 'Part01Face001', 'Part01Eye001', 'Part01Mouth001'],
};

/** Parameters worth driving directly; every sample model defines them. */
const PARAMETER_IDS = [
  'ParamAngleX',
  'ParamAngleY',
  'ParamAngleZ',
  'ParamEyeLOpen',
  'ParamEyeROpen',
  'ParamMouthOpenY',
  'ParamBodyAngleX',
  'ParamBreath',
];

const OPTIONS = [
  { key: 'physics', label: '物理' },
  { key: 'pose', label: '姿态' },
  { key: 'blink', label: '眨眼' },
  { key: 'breath', label: '呼吸' },
  { key: 'look', label: '目光跟随' },
  { key: 'idle', label: '自动待机' },
] as const;

type OptionKey = (typeof OPTIONS)[number]['key'];
type OptionState = Record<OptionKey, boolean>;
type MainTab = 'show' | 'tune' | 'diagnose';
type TuneTab = 'parameters' | 'parts';

const DEFAULT_OPTIONS: OptionState = {
  physics: true,
  pose: true,
  blink: true,
  breath: true,
  look: true,
  idle: true,
};

function chunk<T>(items: readonly T[], size: number): T[][] {
  const rows: T[][] = [];
  for (let index = 0; index < items.length; index += size) {
    rows.push(items.slice(index, index + size));
  }

  return rows;
}

function modelKey(src: string): string {
  return MODELS.find((model) => model.value === src)?.label ?? '';
}

function normalize(value: number, min: number, max: number): number {
  if (max <= min) return 0.5;

  return Math.max(0, Math.min(1, (value - min) / (max - min)));
}

function formatParameter(
  parameter: Live2dMetadata['parameters'][number],
  normalized: number,
): string {
  const value = parameter.min + normalized * (parameter.max - parameter.min);

  return `${value.toFixed(1)} / [${parameter.min}, ${parameter.max}]`;
}

/** Largest scale that keeps the whole canvas inside the view box. */
function fitScale(metadata: Live2dMetadata | null): number {
  if (!metadata) return 0.16;

  return Math.min(VIEW_WIDTH / metadata.canvas.width, VIEW_HEIGHT / metadata.canvas.height) * 0.94;
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

/**
 * Normalizes a wheel delta to zoom steps, one wheel line per step. Line mode
 * reports whole lines while pixel mode reports a notch's worth of pixels, so the
 * two need separate scales.
 */
function wheelSteps(event: MoyuWheelEvent): number {
  switch (event.deltaMode) {
    case 'line':
      return event.deltaY;
    case 'page':
      return event.deltaY * 20;
    default:
      return event.deltaY / 100;
  }
}

function groupMotions(metadata: Live2dMetadata): { group: string; indices: number[] }[] {
  const groups = new Map<string, number[]>();
  for (const motion of metadata.motions) {
    groups.set(motion.group, [...(groups.get(motion.group) ?? []), motion.index]);
  }

  return [...groups].map(([group, indices]) => ({
    group,
    indices: [...indices].sort((a, b) => a - b),
  }));
}

/** Lays out a flat list of items as rows of callbacks. */
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

export function Live2dPage() {
  const { t } = useLingui();
  const live2dRef = useRef<Node>(null);

  const [src, setSrc] = useState<string>(MODELS[0].value);
  const [options, setOptions] = useState<OptionState>(DEFAULT_OPTIONS);
  const [metadata, setMetadata] = useState<Live2dMetadata | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [log, setLog] = useState<string[]>([]);
  const [parameterValues, setParameterValues] = useState<Record<string, number>>({});
  const [partId, setPartId] = useState<string>('');
  const [partOpacity, setPartOpacity] = useState(1);
  const [hitResult, setHitResult] = useState<string | null>(null);
  /** Pan offset of the model inside the viewport, in stage units. */
  const [pan, setPan] = useState({ x: 0, y: 0 });
  /** Zoom multiplier over the fit scale. */
  const [zoom, setZoom] = useState(1);
  const [panning, setPanning] = useState(false);
  /** Last pointer position in viewport coordinates, `null` before the first move. */
  const pointerRef = useRef<{ x: number; y: number } | null>(null);
  /** Press that may turn into a pan, held until the pointer moves or is released. */
  const dragRef = useRef<{ clientX: number; clientY: number; panX: number; panY: number } | null>(
    null,
  );
  /** Whether the current press has travelled far enough to be a pan and not a click. */
  const dragMovedRef = useRef(false);

  const [mainTab, setMainTab] = useState<MainTab>('show');
  const [tuneTab, setTuneTab] = useState<TuneTab>('parameters');

  const [motionGroup, setMotionGroup] = useState(0);
  const [motionPage, setMotionPage] = useState(0);
  const [expressionPage, setExpressionPage] = useState(0);
  const [logPage, setLogPage] = useState(0);

  const showScroll = useScrollView({ viewportHeight: TAB_BODY_HEIGHT });
  const tuneScroll = useScrollView({ viewportHeight: TAB_BODY_HEIGHT - 60 });
  const diagnoseScroll = useScrollView({ viewportHeight: TAB_BODY_HEIGHT });

  const partIds = PART_IDS[modelKey(src)] ?? [];
  const currentPartId = partId || partIds[0] || '';
  const baseScale = fitScale(metadata);
  const scale = baseScale * zoom;
  const canvasWidth = metadata?.canvas.width ?? 0;
  const canvasHeight = metadata?.canvas.height ?? 0;

  const pushLog = useCallback((text: string) => {
    setLog((entries) => [text, ...entries].slice(0, LOG_LIMIT));
  }, []);

  const run = useCallback(
    (command: Live2dCommand, description: string) => {
      live2dRef.current?.executeCommand(command);
      pushLog(description);
    },
    [pushLog],
  );

  const parameters = useMemo(
    () => (metadata?.parameters ?? []).filter((parameter) => PARAMETER_IDS.includes(parameter.id)),
    [metadata],
  );
  const motionGroups = useMemo(() => (metadata ? groupMotions(metadata) : []), [metadata]);
  const expressions = metadata?.expressions ?? [];
  const logPages = Math.max(1, Math.ceil(log.length / LOG_PAGE_SIZE));

  const modelWidth = canvasWidth * scale;
  /** Model placement inside the viewport: centered across, `VIEW_TOP` units down. */
  const modelX = (VIEW_WIDTH - modelWidth) / 2 + pan.x;
  const modelY = VIEW_TOP + pan.y;

  /**
   * Pointer position in viewport coordinates. A move over the model reports canvas
   * coordinates, which the current placement converts back, so the anchor is right
   * over the model and over the bare viewport alike.
   */
  const toViewport = useCallback(
    (event: MoyuMouseEvent) => {
      if (event.targetId === live2dRef.current?.nodeId) {
        return { x: modelX + event.offsetX * scale, y: modelY + event.offsetY * scale };
      }

      return { x: event.offsetX, y: event.offsetY };
    },
    [modelX, modelY, scale],
  );

  /**
   * Zooms around the pointer, keeping the model point under it in place. The
   * pointer's canvas point is what the pan has to hold as the scale changes.
   */
  const handleWheel = useCallback(
    (event: MoyuWheelEvent) => {
      if (!metadata) return;
      event.stopPropagation();

      const steps = wheelSteps(event);
      if (steps === 0) return;

      const nextZoom = clamp(zoom * (1 + steps * ZOOM_PER_STEP), MIN_ZOOM, MAX_ZOOM);
      if (nextZoom === zoom) return;

      const anchor = pointerRef.current ?? { x: VIEW_WIDTH / 2, y: VIEW_HEIGHT / 2 };
      const localX = (anchor.x - modelX) / scale;
      const localY = (anchor.y - modelY) / scale;
      const nextScale = baseScale * nextZoom;
      setZoom(nextZoom);
      setPan({
        x: anchor.x - localX * nextScale - (VIEW_WIDTH - canvasWidth * nextScale) / 2,
        y: anchor.y - localY * nextScale - VIEW_TOP,
      });
    },
    [baseScale, canvasWidth, metadata, modelX, modelY, scale, zoom],
  );

  const handleViewportMove = useCallback(
    (event: MoyuMouseEvent) => {
      pointerRef.current = toViewport(event);
    },
    [toViewport],
  );

  const handlePanStart = useCallback(
    (event: MoyuMouseEvent | MoyuTouchEvent) => {
      if (!metadata) return;
      if ('button' in event && event.button !== 0) return;

      dragRef.current = {
        clientX: event.clientX,
        clientY: event.clientY,
        panX: pan.x,
        panY: pan.y,
      };
      dragMovedRef.current = false;
      setPanning(true);
    },
    [metadata, pan],
  );

  /**
   * Panning continues wherever the pointer goes, so the move and release are
   * watched on the window rather than on the viewport.
   */
  const handlePanMove = useCallback((event: MoyuMouseEvent | MoyuTouchEvent) => {
    const drag = dragRef.current;
    if (drag === null) return;

    const deltaX = event.clientX - drag.clientX;
    const deltaY = event.clientY - drag.clientY;
    if (!dragMovedRef.current && Math.hypot(deltaX, deltaY) < DRAG_THRESHOLD) return;

    dragMovedRef.current = true;
    setPan({ x: drag.panX + deltaX, y: drag.panY + deltaY });
  }, []);

  const handlePanEnd = useCallback(() => {
    dragRef.current = null;
    setPanning(false);
  }, []);

  useEffect(() => {
    const cleanups = [
      addEventListener('mousemove', handlePanMove),
      addEventListener('mouseup', handlePanEnd),
      addEventListener('touchmove', handlePanMove),
      addEventListener('touchend', handlePanEnd),
      addEventListener('touchcancel', handlePanEnd),
    ];

    return () => {
      for (const cleanup of cleanups) {
        cleanup();
      }
    };
  }, [handlePanMove, handlePanEnd]);

  const handleResetView = useCallback(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, []);

  const handleReady = useCallback(
    (event: Extract<Live2dEvent, { type: 'ready' }>) => {
      setMetadata(event);
      setError(null);
      setParameterValues(
        Object.fromEntries(
          event.parameters
            .filter((parameter) => PARAMETER_IDS.includes(parameter.id))
            .map((parameter) => [
              parameter.id,
              normalize(parameter.default, parameter.min, parameter.max),
            ]),
        ),
      );
      setPartId(PART_IDS[modelKey(src)]?.[0] ?? '');
      setPartOpacity(1);
      setMotionGroup(0);
      setMotionPage(0);
      setExpressionPage(0);
      setLogPage(0);
      // A new canvas starts from the fitted view again.
      setZoom(1);
      setPan({ x: 0, y: 0 });
      pointerRef.current = null;
      pushLog(
        `ready：画布 ${event.canvas.width}×${event.canvas.height}px、${event.motions.length} 动作、` +
          `${event.expressions.length} 表情、${event.parameters.length} 参数、命中区 ${event.hitAreas.join('/')}`,
      );
    },
    [pushLog, src],
  );

  const handleModelChange = useCallback((value: string) => {
    live2dRef.current?.executeCommand({ subCommand: 'stopAllMotions' });
    setSrc(value);
    setMetadata(null);
    setError(null);
    setLog([]);
    setHitResult(null);
  }, []);

  const setOption = useCallback((key: OptionKey, value: boolean) => {
    setOptions((current) => ({ ...current, [key]: value }));
  }, []);

  const handleParameter = useCallback(
    (id: string, normalized: number) => {
      setParameterValues((current) => ({ ...current, [id]: normalized }));
      const parameter = metadata?.parameters.find((item) => item.id === id);
      if (!parameter) return;

      live2dRef.current?.executeCommand({
        subCommand: 'setParameter',
        id,
        value: parameter.min + normalized * (parameter.max - parameter.min),
      });
    },
    [metadata],
  );

  const currentGroup = motionGroups[Math.min(motionGroup, motionGroups.length - 1)];
  const motionPages = currentGroup
    ? Math.max(1, Math.ceil(currentGroup.indices.length / MOTION_PAGE_SIZE))
    : 1;
  const motionSlice = currentGroup
    ? currentGroup.indices.slice(motionPage * MOTION_PAGE_SIZE, (motionPage + 1) * MOTION_PAGE_SIZE)
    : [];
  const expressionPages = Math.max(1, Math.ceil(expressions.length / EXPRESSION_PAGE_SIZE));
  const expressionSlice = expressions.slice(
    expressionPage * EXPRESSION_PAGE_SIZE,
    (expressionPage + 1) * EXPRESSION_PAGE_SIZE,
  );
  const logSlice = log.slice(logPage * LOG_PAGE_SIZE, (logPage + 1) * LOG_PAGE_SIZE);
  const motionSummary =
    motionGroups.map((group) => `${group.group} ×${group.indices.length}`).join('、') || '无';

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
                text={metadata ? `${canvasWidth} × ${canvasHeight} px` : t`未加载`}
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
                    ? t`模型已就绪；右侧画面与官方样例一致即通过。`
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
            { value: 'show', label: t`演出验证` },
            { value: 'tune', label: t`精细调校` },
            { value: 'diagnose', label: t`接入诊断` },
          ]}
        />

        <Panel width={COLUMN_WIDTH} height={MAIN_TAB_HEIGHT}>
          {mainTab === 'show' ? (
            <ScrollView width={SCROLL_WIDTH} height={TAB_BODY_HEIGHT} controller={showScroll} contentProps={{ gap: 12 }}>
              <vbox gap={10}>
                {chunk(OPTIONS, 3).map((row, index) => (
                  <hbox key={index} gap={28} alignItems="center">
                    {row.map((item) => (
                      <hbox key={item.key} gap={10} alignItems="center">
                        <Checkbox
                          checked={options[item.key]}
                          onCheckedChange={(checked) => setOption(item.key, checked)}
                          uncheckedSprite={CHECKBOX_UNCHECKED_SPRITE}
                          checkedSprite={CHECKBOX_CHECKED_SPRITE}
                        />
                        <text {...TEXT.body} text={item.label} />
                      </hbox>
                    ))}
                  </hbox>
                ))}
              </vbox>

              <text {...GROUP_LABEL} text={t`动作`} />
              {motionGroups.length === 0 ? (
                <text {...TEXT.caption} text={t`加载模型后列出动作组。`} />
              ) : (
                <vbox gap={12}>
                  <hbox gap={10} alignItems="center" zIndex={2}>
                    <Select
                      value={String(currentGroup?.group ?? '')}
                      onValueChange={(value) => {
                        setMotionGroup(motionGroups.findIndex((group) => group.group === value));
                        setMotionPage(0);
                      }}
                      options={motionGroups.map((group) => ({
                        text: `${group.group}（${group.indices.length}）`,
                        value: group.group,
                      }))}
                      trigger={{ ...SELECT_TRIGGER, targetWidth: 300 }}
                      list={{ ...SELECT_LIST, targetWidth: 300 }}
                      option={{ ...SELECT_OPTION, targetWidth: 294 }}
                      textStyle={BUTTON_TEXT_STYLE}
                    />
                  </hbox>
                  <ItemRows
                    items={motionSlice.map(String)}
                    render={(index) => (
                      <Button
                        key={`${currentGroup?.group}-${index}`}
                        sprite={SMALL_BUTTON}
                        text={`#${index}`}
                        textStyle={BUTTON_TEXT_STYLE}
                        onPress={() =>
                          run(
                            {
                              subCommand: 'startMotion',
                              group: currentGroup?.group ?? '',
                              index: Number(index),
                              loop: false,
                            },
                            `startMotion ${currentGroup?.group} #${index}`,
                          )
                        }
                      />
                    )}
                  />
                  <Pager page={motionPage} pages={motionPages} onChange={setMotionPage} />
                  <text {...TEXT.caption} text={t`编号即 motion3.json 索引；优先级 Normal。`} />
                  <hbox gap={10}>
                    <Button
                      sprite={WIDE_BUTTON}
                      text={t`停止全部动作`}
                      textStyle={BUTTON_TEXT_STYLE}
                      onPress={() => run({ subCommand: 'stopAllMotions' }, 'stopAllMotions')}
                    />
                  </hbox>
                </vbox>
              )}

              <text {...GROUP_LABEL} text={t`表情`} />
              {expressions.length === 0 ? (
                <text {...TEXT.caption} text={t`该模型没有表情，或模型尚未就绪。`} />
              ) : (
                <vbox gap={12}>
                  <ItemRows
                    items={expressionSlice.map((expression) => expression.name)}
                    perRow={3}
                    render={(name) => (
                      <Button
                        key={name}
                        sprite={EXPRESSION_BUTTON}
                        text={name}
                        textStyle={BUTTON_TEXT_STYLE}
                        onPress={() =>
                          run({ subCommand: 'setExpression', name }, `setExpression ${name}`)
                        }
                      />
                    )}
                  />
                  <Pager page={expressionPage} pages={expressionPages} onChange={setExpressionPage} />
                  <text {...TEXT.caption} text={t`停止表情后回到默认表情，动作不受影响。`} />
                  <hbox gap={10}>
                    <Button
                      sprite={WIDE_BUTTON}
                      text={t`停止表情`}
                      textStyle={BUTTON_TEXT_STYLE}
                      onPress={() => run({ subCommand: 'stopExpression' }, 'stopExpression')}
                    />
                  </hbox>
                </vbox>
              )}
            </ScrollView>
          ) : null}

          {mainTab === 'tune' ? (
            <vbox gap={12}>
              <SectionTabs
                value={tuneTab}
                onChange={setTuneTab}
                tabWidth={180}
                options={[
                  { value: 'parameters', label: t`参数` },
                  { value: 'parts', label: t`部件不透明度` },
                ]}
              />

              {tuneTab === 'parameters' ? (
                <ScrollView
                  width={SCROLL_WIDTH}
                  height={TAB_BODY_HEIGHT - 60}
                  controller={tuneScroll}
                  contentProps={{ gap: 10 }}
                >
                  {parameters.length === 0 ? (
                    <text {...TEXT.caption} text={t`加载模型后列出模型实际定义的常用参数。`} />
                  ) : (
                    parameters.map((parameter) => (
                      <hbox key={parameter.id} gap={12} alignItems="center">
                        <text {...GROUP_LABEL} text={parameter.id} boxWidth={190} />
                        <Slider
                          value={parameterValues[parameter.id] ?? 0.5}
                          onValueChange={(value) => handleParameter(parameter.id, value)}
                          track={{ ...SLIDER_TRACK, targetWidth: 300 }}
                          thumb={SLIDER_THUMB}
                        />
                        <vbox width={190}>
                          <text
                            {...TEXT.caption}
                            text={formatParameter(parameter, parameterValues[parameter.id] ?? 0.5)}
                          />
                        </vbox>
                      </hbox>
                    ))
                  )}
                  <text
                    {...TEXT.caption}
                    text={t`覆盖值每帧在效果链之后重放，动作运行时也保持生效。`}
                  />
                  <hbox gap={10}>
                    <Button
                      sprite={WIDE_BUTTON}
                      text={t`重置参数`}
                      textStyle={BUTTON_TEXT_STYLE}
                      onPress={() => {
                        run({ subCommand: 'resetParameters' }, 'resetParameters');
                        setParameterValues({});
                      }}
                    />
                  </hbox>
                </ScrollView>
              ) : null}

              {tuneTab === 'parts' ? (
                <vbox gap={16}>
                  <hbox gap={10} zIndex={2}>
                    <Select
                      value={currentPartId}
                      onValueChange={(value) => {
                        setPartId(value);
                        run(
                          { subCommand: 'setPartOpacity', id: value, value: partOpacity },
                          `setPartOpacity ${value} ${partOpacity.toFixed(2)}`,
                        );
                      }}
                      options={partIds.map((id) => ({ text: id, value: id }))}
                      trigger={{ ...SELECT_TRIGGER, targetWidth: 380 }}
                      list={{ ...SELECT_LIST, targetWidth: 380 }}
                      option={{ ...SELECT_OPTION, targetWidth: 374 }}
                      textStyle={BUTTON_TEXT_STYLE}
                    />
                  </hbox>
                  <hbox gap={16} alignItems="center">
                    <Slider
                      value={partOpacity}
                      onValueChange={(value) => {
                        setPartOpacity(value);
                        if (!metadata || currentPartId === '') return;

                        live2dRef.current?.executeCommand({
                          subCommand: 'setPartOpacity',
                          id: currentPartId,
                          value,
                        });
                      }}
                      track={{ ...SLIDER_TRACK, targetWidth: 400 }}
                      thumb={SLIDER_THUMB}
                    />
                    <text {...TEXT.body} text={partOpacity.toFixed(2)} fillColor={COLOR.accent} />
                  </hbox>
                  <text
                    {...TEXT.caption}
                    text={t`ready 元数据不含部件列表，这里用样例模型的已知部件 id。`}
                  />
                </vbox>
              ) : null}
            </vbox>
          ) : null}

          {mainTab === 'diagnose' ? (
            <ScrollView
              width={SCROLL_WIDTH}
              height={TAB_BODY_HEIGHT}
              controller={diagnoseScroll}
              contentProps={{ gap: 12 }}
            >
              <text {...GROUP_LABEL} text={t`插件状态`} />
              <InfoRow
                label={t`状态`}
                value={error ?? t`正常：插件已加载，模型已就绪。`}
                accent={error !== null}
              />

              <text {...GROUP_LABEL} text={t`模型放置`} />
              <InfoRow label={t`模型`} value="assets/live2d/<名称>/<名称>.model3.json" />
              <InfoRow label={t`插件`} value="moyu_live2d_plugin 放到引擎可执行文件旁边" />

              <text {...GROUP_LABEL} text={t`模型元信息`} />
              {metadata ? (
                <vbox gap={10}>
                  <InfoRow
                    label={t`画布`}
                    value={`${canvasWidth} × ${canvasHeight} px`}
                    accent
                  />
                  <InfoRow
                    label={t`画布原点`}
                    value={`(${metadata.canvas.originX}, ${metadata.canvas.originY})`}
                  />
                  <InfoRow label={t`每单位像素`} value={String(metadata.canvas.pixelsPerUnit)} />
                  <InfoRow label={t`动作`} value={`${metadata.motions.length} 个：${motionSummary}`} />
                  <InfoRow label={t`表情`} value={`${expressions.length} 个`} />
                  <InfoRow label={t`参数`} value={`${metadata.parameters.length} 个`} />
                  <InfoRow
                    label={t`命中区`}
                    value={metadata.hitAreas.join('、') || t`（无）`}
                  />
                </vbox>
              ) : (
                <text {...TEXT.caption} text={t`尚未收到 ready 事件。`} />
              )}

              <text {...GROUP_LABEL} text={t`事件日志`} />
              {log.length === 0 ? (
                <text {...TEXT.caption} text={t`等待事件。`} />
              ) : (
                logSlice.map((entry, index) => (
                  <text
                    key={`${logPage}-${index}-${entry}`}
                    {...TEXT.caption}
                    boxWidth={SCROLL_WIDTH - 20}
                    lineHeight={1.3}
                    text={entry}
                  />
                ))
              )}
              <Pager page={logPage} pages={logPages} onChange={setLogPage} />
              <text {...TEXT.caption} text={t`保留最近 ${LOG_LIMIT} 条。`} />
              <hbox gap={10}>
                <Button
                  sprite={WIDE_BUTTON}
                  text={t`清空日志`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() => {
                    setLog([]);
                    setLogPage(0);
                  }}
                />
              </hbox>
            </ScrollView>
          ) : null}
        </Panel>
      </vbox>

      <vbox gap={COLUMN_GAP} width={COLUMN_WIDTH}>
        <Panel
          title={t`渲染结果`}
          width={COLUMN_WIDTH}
          height={PREVIEW_HEIGHT}
          note={t`滚轮缩放，拖拽平移；点击画面做命中测试。`}
        >
          {/* Viewport: the clip rejects pointers outside it, so wheel and drag
              stop at the panel edge on their own. */}
          <clip width={VIEW_WIDTH} height={VIEW_HEIGHT}>
            <container
              label="live2d viewport"
              cursor={panning ? 'grabbing' : 'grab'}
              onMouseMove={handleViewportMove}
              onMouseDown={handlePanStart}
              onMouseUp={handlePanEnd}
              onTouchStart={handlePanStart}
              onWheel={handleWheel}
            >
              <live2d
                ref={live2dRef}
                src={src}
                x={modelX}
                y={modelY}
                scale={scale}
                physics={options.physics}
                pose={options.pose}
                blink={options.blink}
                breath={options.breath}
                look={options.look}
                idle={options.idle}
                onClick={(event) => {
                  if (!metadata || dragMovedRef.current) return;
                  const x = event.offsetX;
                  const y = event.offsetY;
                  const matched = metadata.hitAreas.filter(
                    (area) =>
                      live2dRef.current?.executeCommand({ subCommand: 'hitTest', area, x, y }) ===
                      true,
                  );
                  setHitResult(
                    `画布坐标 (${Math.round(x)}, ${Math.round(y)}) → ${
                      matched.length > 0 ? matched.join(', ') : t`未命中任何区域`
                    }`,
                  );
                }}
                onReady={handleReady}
                onMotionFinished={(event) =>
                  pushLog(`motionFinished ${event.group} #${event.index}`)
                }
                onMotionEvent={(event) => pushLog(`motionEvent ${event.value}`)}
                onBlendModeUnsupported={(event) => pushLog(`blendModeUnsupported ${event.value}`)}
                onError={(event) => {
                  setError(t`错误 ${event.code}：${event.message}`);
                  pushLog(`error ${event.code}: ${event.message}`);
                }}
              />
            </container>
          </clip>
          <container x={VIEW_WIDTH - RESET_BUTTON_WIDTH} y={-36} zIndex={2}>
            <Button
              sprite={{ ...BUTTON_SPRITE, targetWidth: RESET_BUTTON_WIDTH, targetHeight: 42 }}
              text={t`重置视角`}
              textStyle={BUTTON_TEXT_STYLE}
              disabled={zoom === 1 && pan.x === 0 && pan.y === 0}
              onPress={handleResetView}
            />
          </container>
        </Panel>

        <Panel title={t`点击反馈`} width={COLUMN_WIDTH} height={DOCK_HEIGHT}>
          <vbox gap={12}>
            <hbox gap={10}>
              {(metadata?.hitAreas ?? []).map((area) => (
                <Button
                  key={area}
                  sprite={{ ...SMALL_BUTTON, targetWidth: 170 }}
                  text={`${area} ${t`中心`}`}
                  textStyle={BUTTON_TEXT_STYLE}
                  onPress={() => {
                    const hit = live2dRef.current?.executeCommand({
                      subCommand: 'hitTest',
                      area,
                      x: canvasWidth / 2,
                      y: canvasHeight / 2,
                    });
                    setHitResult(
                      `${area} @ (${Math.round(canvasWidth / 2)}, ${Math.round(
                        canvasHeight / 2,
                      )}) → ${hit === true ? t`命中` : t`未命中`}`,
                    );
                  }}
                />
              ))}
            </hbox>
            <text
              {...TEXT.caption}
              boxWidth={COLUMN_WIDTH - 60}
              lineHeight={1.3}
              fillColor={hitResult ? COLOR.accent : COLOR.dim}
              text={hitResult ?? t`尚未执行命中测试；点击右侧画面或上面的按钮。`}
            />
          </vbox>
        </Panel>
      </vbox>
    </hbox>
  );
}
