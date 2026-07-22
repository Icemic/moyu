import { Button, executePluginCommand, Select, type SteamCommand } from '@momoyu-ink/kit';
import { useState } from 'react';
import { Panel, SectionTabs } from '../components/chrome';
import { BUTTON_SPRITE, BUTTON_TEXT_STYLE, COLOR, SELECT_LIST, SELECT_OPTION, SELECT_TRIGGER, TEXT } from '../theme';

const APP_ID = 480;
const PROGRESS_ACHIEVEMENT = 'ACH_WIN_100_GAMES';
const ACHIEVEMENTS = [
  'ACH_WIN_ONE_GAME',
  'ACH_WIN_100_GAMES',
  'ACH_TRAVEL_FAR_ACCUM',
  'ACH_TRAVEL_FAR_SINGLE',
  'NEW_ACHIEVEMENT_0_4',
] as const;

const TABS = [
  { value: 'achievement', label: 'Achievement' },
  { value: 'apps-overlay', label: 'Apps / Overlay' },
  { value: 'stats', label: 'Stats' },
  { value: 'other', label: 'User / Workshop / Timeline' },
] as const;

type SteamTab = (typeof TABS)[number]['value'];

const SMALL_BUTTON = {
  ...BUTTON_SPRITE,
  targetWidth: 300,
  targetHeight: 52,
};

function ResultPanel({ command, result }: { command: string; result: string }) {
  return (
    <Panel title="最近一次调用" width={1460} height={200} note="显示实际提交的 subCommand 与同步返回值或错误。">
      <vbox gap={10}>
        <text {...TEXT.body} text={command || '尚未调用'} />
        <text {...TEXT.caption} text={result || '点击按钮开始测试。'} boxWidth={1400} lineHeight={28} />
      </vbox>
    </Panel>
  );
}

function AchievementSection({ run }: { run: (command: SteamCommand) => void }) {
  const [achievement, setAchievement] = useState<string>(ACHIEVEMENTS[0]);
  const [progress, setProgress] = useState(0);

  return (
    <vbox gap={24}>
      <Panel title="Spacewar Achievement" width={1460} height={370} note="API name 来自 App ID 480 的真实后台配置。">
        <vbox gap={18}>
          <Select
            value={achievement}
            onValueChange={setAchievement}
            options={ACHIEVEMENTS.map((name) => ({ text: name, value: name }))}
            trigger={{ ...SELECT_TRIGGER, targetWidth: 520 }}
            list={{ ...SELECT_LIST, targetWidth: 520 }}
            option={{ ...SELECT_OPTION, targetWidth: 512 }}
            textStyle={{ ...BUTTON_TEXT_STYLE, fontSize: 18, glyphGridSize: 18 }}
          />
          <hbox gap={16}>
            <Button
              sprite={SMALL_BUTTON}
              text="achievementGet"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'achievementGet', name: achievement })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="achievementSet"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'achievementSet', name: achievement })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="achievementClear"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'achievementClear', name: achievement })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="achievementClearAll"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'achievementClearAll' })}
            />
          </hbox>
          <hbox gap={16}>
            <Button
              sprite={SMALL_BUTTON}
              text={`indicateProgress ${progress}/100`}
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => {
                const current = progress >= 90 ? 10 : progress + 10;
                setProgress(current);
                run({ subCommand: 'achievementIndicateProgress', name: PROGRESS_ACHIEVEMENT, current, max: 100 });
              }}
            />
          </hbox>
          <text {...TEXT.caption} text="进度通知不保存数值；真实进度通过 Stats 页的 NumWins 读写。" />
        </vbox>
      </Panel>
    </vbox>
  );
}

function AppsOverlaySection({ run }: { run: (command: SteamCommand) => void }) {
  return (
    <vbox gap={24}>
      <Panel title="Apps · App ID 480" width={1460} height={200}>
        <vbox gap={18}>
          <hbox gap={16}>
            <Button
              sprite={SMALL_BUTTON}
              text="appsGetAppBuildId"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'appsGetAppBuildId' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="appsGetCurrentBetaName"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'appsGetCurrentBetaName' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="appsGetGameLanguage"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'appsGetCurrentGameLanguage' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="appsGetSteamUiLanguage"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'appsGetSteamUiLanguage' })}
            />
          </hbox>
          <Button
            sprite={SMALL_BUTTON}
            text="appsIsSubscribedApp(480)"
            textStyle={BUTTON_TEXT_STYLE}
            onPress={() => run({ subCommand: 'appsIsSubscribedApp', appId: APP_ID })}
          />
        </vbox>
      </Panel>
      <Panel title="Overlay" width={1460} height={250}>
        <vbox gap={18}>
          <hbox gap={16}>
            <Button
              sprite={SMALL_BUTTON}
              text="overlayIsEnabled"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'overlayIsEnabled' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="overlayNeedsPresent"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'overlayNeedsPresent' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="overlayActivate(friends)"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'overlayActivate', dialog: 'friends' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="overlayActivate(settings)"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'overlayActivate', dialog: 'settings' })}
            />
          </hbox>
          <hbox gap={16}>
            <Button
              sprite={SMALL_BUTTON}
              text="overlayActivate(stats)"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'overlayActivate', dialog: 'stats' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="overlayActivate(achievements)"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'overlayActivate', dialog: 'achievements' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="notification: topRight"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'overlaySetNotificationPosition', position: 'topRight' })}
            />
          </hbox>
          <text
            {...TEXT.caption}
            text="未提供测试按钮：DLC、overlayActivateToStore、overlayActivateToWebPage（App 480 无已确认参数）。"
          />
        </vbox>
      </Panel>
    </vbox>
  );
}

function StatsSection({ run }: { run: (command: SteamCommand) => void }) {
  const [wins, setWins] = useState(0);

  return (
    <vbox gap={24}>
      <Panel
        title="UserStats · Spacewar"
        width={1460}
        height={360}
        note="Achievement 页面显示的数值进度来自后台绑定的 Stat，写入后需要 StoreStats。"
      >
        <vbox gap={18}>
          <hbox gap={16}>
            <Button
              sprite={SMALL_BUTTON}
              text="statsListAchievements"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'statsListAchievements' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="statsGetAchievement"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'statsGetAchievement', name: 'ACH_WIN_ONE_GAME' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="statsSetAchievement"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'statsSetAchievement', name: 'ACH_WIN_ONE_GAME' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="statsClearAchievement"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'statsClearAchievement', name: 'ACH_WIN_ONE_GAME' })}
            />
          </hbox>
          <hbox gap={16}>
            <Button
              sprite={SMALL_BUTTON}
              text="statsGetIntStat(NumWins)"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'statsGetIntStat', name: 'NumWins' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text={`statsSetIntStat(${wins})`}
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => {
                const value = wins + 1;
                setWins(value);
                run({ subCommand: 'statsSetIntStat', name: 'NumWins', value });
              }}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="statsIndicateProgress"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() =>
                run({
                  subCommand: 'statsIndicateAchievementProgress',
                  name: PROGRESS_ACHIEVEMENT,
                  current: Math.min(99, wins),
                  max: 100,
                })
              }
            />
            <Button
              sprite={SMALL_BUTTON}
              text="statsStoreStats"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'statsStoreStats' })}
            />
          </hbox>
          <hbox gap={16}>
            <Button
              sprite={SMALL_BUTTON}
              text="get FeetTraveled"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'statsGetFloatStat', name: 'FeetTraveled' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="set FeetTraveled: 2640"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'statsSetFloatStat', name: 'FeetTraveled', value: 2640 })}
            />
          </hbox>
          <text {...TEXT.caption} text="ACH_TRAVEL_FAR_ACCUM · 累计 Stat，只能推进；2640 / 5280 feet" />
        </vbox>
      </Panel>
    </vbox>
  );
}

function OtherSection({ run }: { run: (command: SteamCommand) => void }) {
  return (
    <vbox gap={24}>
      <Panel title="User / Workshop" width={1460} height={240}>
        <vbox gap={18}>
          <hbox gap={16}>
            <Button
              sprite={SMALL_BUTTON}
              text="userGetAccountId"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'userGetAccountId' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="userGetCSteamId"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'userGetCSteamId' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="userGetPersonaName"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'userGetPersonaName' })}
            />
            <Button
              sprite={SMALL_BUTTON}
              text="userGetGameBadgeLevel"
              textStyle={BUTTON_TEXT_STYLE}
              onPress={() => run({ subCommand: 'userGetGameBadgeLevel', series: 1, foil: false })}
            />
          </hbox>
          <Button
            sprite={SMALL_BUTTON}
            text="workshopGetSubscribedItems"
            textStyle={BUTTON_TEXT_STYLE}
            onPress={() => run({ subCommand: 'workshopGetSubscribedItems', includeDisabled: false })}
          />
          <text {...TEXT.caption} text="未提供测试按钮：workshopGetSubscribedItemPath（没有已确认的 item ID）。" />
        </vbox>
      </Panel>
      <Panel
        title="Timeline / Float Stats"
        width={1460}
        height={200}
        note="App 480 没有可确认的 Timeline 与 float stat 配置，因此不伪造成功参数。"
      >
        <vbox gap={12}>
          <text {...TEXT.body} text="timelineSetStateDescription · timelineAddEvent · timelineClearStateDescription" />
          <text {...TEXT.body} text="statsGetFloatStat · statsSetFloatStat" />
        </vbox>
      </Panel>
    </vbox>
  );
}

export function SteamPage() {
  const [tab, setTab] = useState<SteamTab>('achievement');
  const [lastCommand, setLastCommand] = useState('');
  const [result, setResult] = useState('');

  const run = async (command: SteamCommand) => {
    setLastCommand(command.subCommand);
    try {
      const value = await executePluginCommand('steam', command);
      setResult(value === undefined ? '成功（void）' : JSON.stringify(value));
    } catch (error) {
      setResult(`错误：${error instanceof Error ? error.message : String(error)}`);
    }
  };

  return (
    <vbox gap={24}>
      <text {...TEXT.caption} fillColor={COLOR.accent} text="Steam App ID 480 · Spacewar" />
      <SectionTabs value={tab} onChange={setTab} options={TABS} tabWidth={340} />
      {tab === 'achievement' ? <AchievementSection run={run} /> : null}
      {tab === 'apps-overlay' ? <AppsOverlaySection run={run} /> : null}
      {tab === 'stats' ? <StatsSection run={run} /> : null}
      {tab === 'other' ? <OtherSection run={run} /> : null}
      <ResultPanel command={lastCommand} result={result} />
    </vbox>
  );
}
