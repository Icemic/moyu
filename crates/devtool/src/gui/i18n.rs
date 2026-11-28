//! Complete internationalization strings (ZhCn default, EnUs supported).

use crate::config::{LocalePreference, ThemePreference};
use crate::model::debug::SessionMode;

#[derive(Debug, Clone, Copy)]
pub struct I18n {
    pub locale: LocalePreference,
}

#[allow(dead_code)]
impl I18n {
    pub const fn new(locale: LocalePreference) -> Self {
        Self { locale }
    }

    pub const fn app_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "末语开发工具",
            LocalePreference::EnUs => "Moyu Devtool",
        }
    }

    pub const fn tab_development(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "开发",
            LocalePreference::EnUs => "Development",
        }
    }

    pub const fn tab_engine(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "引擎",
            LocalePreference::EnUs => "Engine",
        }
    }

    pub const fn tab_pack(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "打包",
            LocalePreference::EnUs => "Package",
        }
    }

    pub const fn tab_debug(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "调试",
            LocalePreference::EnUs => "Debug",
        }
    }

    pub const fn settings(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "设置",
            LocalePreference::EnUs => "Settings",
        }
    }

    pub const fn recent_projects(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "最近工程",
            LocalePreference::EnUs => "Recent Projects",
        }
    }

    pub const fn no_project_selected(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "选择工程",
            LocalePreference::EnUs => "Select Project",
        }
    }

    pub const fn new_project(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "新建工程",
            LocalePreference::EnUs => "New Project",
        }
    }

    pub const fn open_existing_project(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "打开已有工程",
            LocalePreference::EnUs => "Open Existing Project",
        }
    }

    pub fn project_not_found_removed(&self, path: &str) -> String {
        match self.locale {
            LocalePreference::ZhCn => {
                format!("工程目录不存在或缺少 index.json，已从列表移除：{path}")
            }
            LocalePreference::EnUs => {
                format!(
                    "Project directory not found or missing index.json; removed from list: {path}"
                )
            }
        }
    }

    pub const fn project_name(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "工程名称",
            LocalePreference::EnUs => "Project Name",
        }
    }

    pub const fn project_name_placeholder(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "例如 my-visual-novel",
            LocalePreference::EnUs => "e.g. my-visual-novel",
        }
    }

    pub const fn scaffold_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "脚手架",
            LocalePreference::EnUs => "Template",
        }
    }

    pub const fn parent_directory(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "父目录",
            LocalePreference::EnUs => "Parent Directory",
        }
    }

    pub const fn browse(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "选择目录",
            LocalePreference::EnUs => "Browse",
        }
    }

    pub const fn create_project_submit(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "创建工程",
            LocalePreference::EnUs => "Create Project",
        }
    }

    pub const fn cancel(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "取消",
            LocalePreference::EnUs => "Cancel",
        }
    }

    pub const fn done(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "完成",
            LocalePreference::EnUs => "Done",
        }
    }

    pub const fn edit(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "编辑",
            LocalePreference::EnUs => "Edit",
        }
    }

    pub const fn save(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "保存",
            LocalePreference::EnUs => "Save",
        }
    }

    pub const fn project_config(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "运行时配置",
            LocalePreference::EnUs => "Runtime Config",
        }
    }

    pub const fn runtime_config_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "运行时配置",
            LocalePreference::EnUs => "Runtime Configuration",
        }
    }

    pub const fn engine_config_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "引擎配置",
            LocalePreference::EnUs => "Engine Configuration",
        }
    }

    pub const fn android_config_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Android 工程配置",
            LocalePreference::EnUs => "Android Configuration",
        }
    }

    pub const fn configure(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "配置",
            LocalePreference::EnUs => "Configure",
        }
    }

    pub const fn cfg_sec_window(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "窗口与基础",
            LocalePreference::EnUs => "Window & Basics",
        }
    }

    pub const fn cfg_sec_features(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "特性与平台",
            LocalePreference::EnUs => "Features & Platform",
        }
    }

    pub const fn cfg_sec_advanced(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "高级渲染与入口设置",
            LocalePreference::EnUs => "Advanced Rendering & Entry",
        }
    }

    pub const fn cfg_app_name(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "应用标识名",
            LocalePreference::EnUs => "App Name",
        }
    }

    pub const fn cfg_window_state(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "初始窗口状态",
            LocalePreference::EnUs => "Window State",
        }
    }

    pub const fn cfg_window_resizable(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "允许缩放窗口",
            LocalePreference::EnUs => "Resizable Window",
        }
    }

    pub const fn cfg_background_color(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "背景色",
            LocalePreference::EnUs => "Background Color",
        }
    }

    pub const fn cfg_font_file(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "字体配置",
            LocalePreference::EnUs => "Fonts",
        }
    }

    pub const fn cfg_add_font(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "添加字体",
            LocalePreference::EnUs => "Add Font",
        }
    }

    pub const fn cfg_font_kind_auto(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "自动",
            LocalePreference::EnUs => "Auto",
        }
    }

    pub const fn cfg_add_cursor(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "添加状态",
            LocalePreference::EnUs => "Add State",
        }
    }

    pub const fn cfg_multi_res_mode(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "多分辨率模式",
            LocalePreference::EnUs => "Multi-Res Mode",
        }
    }

    pub const fn cfg_multi_res_fixed(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "固定倍率",
            LocalePreference::EnUs => "Fixed Scale",
        }
    }

    pub const fn cfg_multi_res_scales(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "候选倍率（逗号分隔）",
            LocalePreference::EnUs => "Candidate Scales",
        }
    }

    pub const fn cfg_multi_res_policy(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "选图策略",
            LocalePreference::EnUs => "Selection Policy",
        }
    }

    pub const fn cfg_enable_gamepads(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "启用手柄",
            LocalePreference::EnUs => "Enable Gamepads",
        }
    }

    pub const fn cfg_skip_splash(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "跳过启动页",
            LocalePreference::EnUs => "Skip Splash",
        }
    }

    pub const fn cfg_show_fps(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "显示 FPS",
            LocalePreference::EnUs => "Show FPS",
        }
    }

    pub const fn cfg_steam_enable(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Steam 集成",
            LocalePreference::EnUs => "Steam Integration",
        }
    }

    pub const fn cfg_steam_required(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "强制要求 Steam",
            LocalePreference::EnUs => "Require Steam",
        }
    }

    pub const fn cfg_steam_restart(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "通过客户端重启",
            LocalePreference::EnUs => "Restart Through Client",
        }
    }

    pub const fn cfg_cursor(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "自定义光标",
            LocalePreference::EnUs => "Custom Cursor",
        }
    }

    pub const fn cfg_entry_redirect(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "入口重定向 (entry)",
            LocalePreference::EnUs => "Entry Redirect",
        }
    }

    pub const fn cfg_autorun(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "自动运行模式",
            LocalePreference::EnUs => "Autorun Mode",
        }
    }

    pub const fn cfg_backend(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "渲染后端",
            LocalePreference::EnUs => "Rendering Backend",
        }
    }

    pub const fn cfg_present_mode(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "垂直同步模式",
            LocalePreference::EnUs => "Present Mode",
        }
    }

    pub const fn cfg_frame_latency(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "最大帧延迟",
            LocalePreference::EnUs => "Max Frame Latency",
        }
    }

    pub const fn cfg_params(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "透传参数 (params)",
            LocalePreference::EnUs => "Custom Params",
        }
    }

    pub const fn cfg_engine_advanced(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "高级源配置（CDN / 锁定版本）",
            LocalePreference::EnUs => "Advanced Source (CDN / Pin Version)",
        }
    }

    pub const fn cfg_engine_pin_version(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "锁定版本（留空自动）",
            LocalePreference::EnUs => "Pin Version (optional)",
        }
    }

    pub const fn cfg_android_basic(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "应用信息",
            LocalePreference::EnUs => "Application Info",
        }
    }

    pub const fn cfg_android_app_name(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "应用名称",
            LocalePreference::EnUs => "App Name",
        }
    }

    pub const fn cfg_android_version_name(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "版本名 (versionName)",
            LocalePreference::EnUs => "Version Name",
        }
    }

    pub const fn cfg_android_version_code(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "版本码 (versionCode)",
            LocalePreference::EnUs => "Version Code",
        }
    }

    pub const fn cfg_android_orientation(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "屏幕方向",
            LocalePreference::EnUs => "Orientation",
        }
    }

    pub const fn cfg_android_icon_signing(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "图标与发布签名（高级）",
            LocalePreference::EnUs => "Icon & Release Signing (Advanced)",
        }
    }

    pub const fn cfg_android_icon_source(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "图标 PNG 相对路径",
            LocalePreference::EnUs => "Icon PNG Path",
        }
    }

    pub const fn cfg_android_icon_bg(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "图标背景色 (#RRGGBB)",
            LocalePreference::EnUs => "Icon Background (#RRGGBB)",
        }
    }

    pub const fn cfg_android_icon_padding(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "前景留白 (0.0–0.5)",
            LocalePreference::EnUs => "Foreground Padding",
        }
    }

    pub const fn cfg_android_keystore(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Keystore 相对路径",
            LocalePreference::EnUs => "Keystore Path",
        }
    }

    pub const fn cfg_android_key_alias(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "签名密钥别名",
            LocalePreference::EnUs => "Key Alias",
        }
    }

    pub const fn window_title_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "窗口标题",
            LocalePreference::EnUs => "Window Title",
        }
    }

    pub const fn entry_script_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "入口脚本",
            LocalePreference::EnUs => "Entry Script",
        }
    }

    pub const fn window_size_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "窗口尺寸",
            LocalePreference::EnUs => "Window Size",
        }
    }

    pub const fn stage_size_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "舞台尺寸",
            LocalePreference::EnUs => "Stage Size",
        }
    }

    pub const fn engine_channel_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "引擎渠道",
            LocalePreference::EnUs => "Engine Channel",
        }
    }

    pub const fn framework_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "框架",
            LocalePreference::EnUs => "Framework",
        }
    }

    pub const fn multi_res_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "多分辨率资源",
            LocalePreference::EnUs => "Multi-Res Assets",
        }
    }

    pub const fn recheck(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "重新检查",
            LocalePreference::EnUs => "Recheck",
        }
    }

    pub const fn check_passed(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "检查通过",
            LocalePreference::EnUs => "Check passed",
        }
    }

    pub fn multi_res_issues(&self, off: usize, scales: usize, missing: usize) -> String {
        match self.locale {
            LocalePreference::ZhCn => {
                format!("发现异常：未启用变体 {off}、未声明倍率 {scales}、缺失基准图 {missing}")
            }
            LocalePreference::EnUs => {
                format!("Issues: off_unused={off}, unused_scales={scales}, missing_base={missing}")
            }
        }
    }

    pub const fn dev_env_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "开发环境",
            LocalePreference::EnUs => "Environment",
        }
    }

    pub const fn redetect(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "重新检测",
            LocalePreference::EnUs => "Re-detect",
        }
    }

    pub const fn project_pm_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "工程包管理器",
            LocalePreference::EnUs => "Package Manager",
        }
    }

    pub const fn found_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "已找到",
            LocalePreference::EnUs => "Found",
        }
    }

    pub const fn not_found_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "未找到",
            LocalePreference::EnUs => "Not found",
        }
    }

    pub const fn node_source_system(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "系统",
            LocalePreference::EnUs => "System",
        }
    }

    pub const fn node_source_managed(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "托管",
            LocalePreference::EnUs => "Managed",
        }
    }

    pub const fn install_managed_node(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "安装 Node LTS",
            LocalePreference::EnUs => "Install Node LTS",
        }
    }

    pub const fn uninstall_managed_node(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "卸载 Node LTS",
            LocalePreference::EnUs => "Uninstall Node LTS",
        }
    }

    pub const fn install_deps(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "安装/重装依赖",
            LocalePreference::EnUs => "Install Dependencies",
        }
    }

    pub const fn dev_server_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Dev Server",
            LocalePreference::EnUs => "Dev Server",
        }
    }

    pub const fn dev_server_stopped(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "未运行",
            LocalePreference::EnUs => "Stopped",
        }
    }

    pub const fn dev_server_starting(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "启动中…",
            LocalePreference::EnUs => "Starting...",
        }
    }

    pub const fn dev_server_start(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "启动",
            LocalePreference::EnUs => "Start",
        }
    }

    pub const fn dev_server_stop(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "停止",
            LocalePreference::EnUs => "Stop",
        }
    }

    pub const fn schema_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Schema",
            LocalePreference::EnUs => "Schema",
        }
    }

    pub const fn scenario_commands(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "剧情命令",
            LocalePreference::EnUs => "Scenario Commands",
        }
    }

    pub const fn game_ui(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "游戏 UI",
            LocalePreference::EnUs => "Game UI",
        }
    }

    pub const fn generate(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "生成",
            LocalePreference::EnUs => "Generate",
        }
    }

    pub fn open_with_editor(&self, editor_name: &str) -> String {
        match self.locale {
            LocalePreference::ZhCn => format!("用 {editor_name} 打开"),
            LocalePreference::EnUs => format!("Open in {editor_name}"),
        }
    }

    pub const fn no_editor_detected(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "未检测到可用工具",
            LocalePreference::EnUs => "No Tool Detected",
        }
    }

    pub const fn detected_editors(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "编辑器",
            LocalePreference::EnUs => "Editors",
        }
    }

    pub const fn detected_terminals(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "终端",
            LocalePreference::EnUs => "Terminals",
        }
    }

    pub const fn detected_ai_tools(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "AI 工具",
            LocalePreference::EnUs => "AI Tools",
        }
    }

    pub const fn open_folder(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "打开文件夹",
            LocalePreference::EnUs => "Open Folder",
        }
    }

    pub const fn run_desktop(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "桌面运行",
            LocalePreference::EnUs => "Desktop Run",
        }
    }

    pub const fn run_browser(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "浏览器运行",
            LocalePreference::EnUs => "Browser Run",
        }
    }

    pub const fn stop_running(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "停止运行",
            LocalePreference::EnUs => "Stop Run",
        }
    }

    pub const fn stop_task(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "取消任务",
            LocalePreference::EnUs => "Cancel Task",
        }
    }

    // Engine view & download dialog
    pub const fn update_latest(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "更新至最新",
            LocalePreference::EnUs => "Update to Latest",
        }
    }

    pub const fn download_version(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "下载版本",
            LocalePreference::EnUs => "Download Version",
        }
    }

    pub const fn download_engine_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "下载引擎版本",
            LocalePreference::EnUs => "Download Engine Version",
        }
    }

    pub const fn download_submit(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "下载",
            LocalePreference::EnUs => "Download",
        }
    }

    pub const fn fetch_manifest(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "刷新清单",
            LocalePreference::EnUs => "Refresh Manifest",
        }
    }

    pub const fn fetching_manifest(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "正在拉取远端版本清单…",
            LocalePreference::EnUs => "Fetching remote manifest...",
        }
    }

    pub const fn load_manifest_hint(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "请先拉取远端清单以选择版本与平台。",
            LocalePreference::EnUs => "Load remote manifest to select version and platforms.",
        }
    }

    pub const fn version_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "版本",
            LocalePreference::EnUs => "Version",
        }
    }

    pub const fn platforms_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "目标平台",
            LocalePreference::EnUs => "Platforms",
        }
    }

    pub const fn current_active(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "当前使用",
            LocalePreference::EnUs => "Current Active",
        }
    }

    pub const fn activated(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "已激活",
            LocalePreference::EnUs => "Active",
        }
    }

    pub const fn local_versions(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "本地版本",
            LocalePreference::EnUs => "Local Versions",
        }
    }

    pub const fn no_local_versions(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "尚未下载任何本地引擎版本。",
            LocalePreference::EnUs => "No local engine versions downloaded yet.",
        }
    }

    pub const fn current_version(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "当前版本",
            LocalePreference::EnUs => "Current",
        }
    }

    pub const fn switch_version(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "切换",
            LocalePreference::EnUs => "Switch",
        }
    }

    // Package view
    pub const fn start_pack(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "开始打包",
            LocalePreference::EnUs => "Start Package",
        }
    }

    pub const fn export_target(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "导出目标",
            LocalePreference::EnUs => "Export Target",
        }
    }

    pub const fn options(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "选项",
            LocalePreference::EnUs => "Options",
        }
    }

    pub const fn target_desktop(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "桌面",
            LocalePreference::EnUs => "Desktop",
        }
    }

    pub const fn target_desktop_desc(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Windows、macOS、Linux",
            LocalePreference::EnUs => "Windows, macOS, Linux",
        }
    }

    pub const fn target_web(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Web",
            LocalePreference::EnUs => "Web",
        }
    }

    pub const fn target_web_desc(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "浏览器静态包",
            LocalePreference::EnUs => "Browser bundle",
        }
    }

    pub const fn target_android(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Android",
            LocalePreference::EnUs => "Android",
        }
    }

    pub const fn target_android_desc(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "APK、AAB、Android 工程",
            LocalePreference::EnUs => "APK, AAB, Android project",
        }
    }

    pub const fn target_framework(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "框架",
            LocalePreference::EnUs => "Framework",
        }
    }

    pub const fn target_framework_desc(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "末语框架 ZIP 包",
            LocalePreference::EnUs => "Moyu Framework ZIP",
        }
    }

    pub const fn pack_platform_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "目标平台",
            LocalePreference::EnUs => "Platform",
        }
    }

    pub const fn pack_android_format_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "导出格式",
            LocalePreference::EnUs => "Format",
        }
    }

    pub fn android_format_name(&self, fmt: &str) -> &'static str {
        match (self.locale, fmt) {
            (LocalePreference::ZhCn, "debug-apk") => "调试 APK (debug-apk)",
            (LocalePreference::ZhCn, "release-apk") => "发布 APK (release-apk)",
            (LocalePreference::ZhCn, "release-aab") => "发布 AAB (release-aab)",
            (LocalePreference::ZhCn, "android-project") => "Android 工程",
            (_, "debug-apk") => "Debug APK",
            (_, "release-apk") => "Release APK",
            (_, "release-aab") => "Release AAB",
            (_, "android-project") => "Android Project",
            _ => "Android",
        }
    }

    pub const fn signing_password_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "签名密码",
            LocalePreference::EnUs => "Signing Password",
        }
    }

    pub const fn android_password_placeholder(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Release 签名密码（debug 留空）",
            LocalePreference::EnUs => "Release signing password",
        }
    }

    pub const fn zip_archive_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "ZIP 压缩包",
            LocalePreference::EnUs => "ZIP Archive",
        }
    }

    pub const fn video_decoder_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "视频解码插件",
            LocalePreference::EnUs => "Video Decoder Plugin",
        }
    }

    pub const fn toggle_on(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "已开启",
            LocalePreference::EnUs => "Enabled",
        }
    }

    pub const fn toggle_off(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "已关闭",
            LocalePreference::EnUs => "Disabled",
        }
    }

    pub const fn output_format_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "输出格式",
            LocalePreference::EnUs => "Output Format",
        }
    }

    pub const fn output_dir_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "输出目录",
            LocalePreference::EnUs => "Output Directory",
        }
    }

    pub const fn pack_complete_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "打包完成",
            LocalePreference::EnUs => "Packaging Complete",
        }
    }

    pub const fn open_output_dir(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "打开输出目录",
            LocalePreference::EnUs => "Open Output Folder",
        }
    }

    // Debug view
    pub const fn dbg_start_session(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "启动会话",
            LocalePreference::EnUs => "Start Session",
        }
    }

    pub const fn dbg_stop_session(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "停止会话",
            LocalePreference::EnUs => "Stop Session",
        }
    }

    pub const fn dbg_session_mode_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "会话模式",
            LocalePreference::EnUs => "Session",
        }
    }

    pub const fn dbg_mode_name(&self, mode: SessionMode) -> &'static str {
        match (self.locale, mode) {
            (LocalePreference::ZhCn, SessionMode::Native) => "桌面引擎",
            (LocalePreference::ZhCn, SessionMode::Web) => "Web 引擎",
            (LocalePreference::ZhCn, SessionMode::Attach) => "附加外部进程",
            (LocalePreference::EnUs, SessionMode::Native) => "Native",
            (LocalePreference::EnUs, SessionMode::Web) => "Web",
            (LocalePreference::EnUs, SessionMode::Attach) => "Attach",
        }
    }

    pub const fn dbg_headless(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "无窗口模式",
            LocalePreference::EnUs => "Headless",
        }
    }

    pub const fn dbg_windowed(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "显示窗口",
            LocalePreference::EnUs => "Windowed",
        }
    }

    pub const fn dbg_status_stopped(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "会话已断开",
            LocalePreference::EnUs => "Stopped",
        }
    }

    pub fn dbg_status_listening(&self, port: u16) -> String {
        match self.locale {
            LocalePreference::ZhCn => format!("等待连接 :{port}"),
            LocalePreference::EnUs => format!("Listening :{port}"),
        }
    }

    pub const fn dbg_status_connected(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "已连接",
            LocalePreference::EnUs => "Connected",
        }
    }

    pub const fn dbg_status_ready(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "已就绪",
            LocalePreference::EnUs => "Ready",
        }
    }

    pub const fn dbg_refresh_tree(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "刷新节点树",
            LocalePreference::EnUs => "Refresh Tree",
        }
    }

    pub const fn dbg_screenshot(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "舞台截图",
            LocalePreference::EnUs => "Screenshot",
        }
    }

    pub const fn dbg_fetch_logs(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "拉取引擎日志",
            LocalePreference::EnUs => "Fetch Logs",
        }
    }

    pub const fn dbg_copy(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "复制",
            LocalePreference::EnUs => "Copy",
        }
    }

    pub const fn dbg_node_tree(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "节点树",
            LocalePreference::EnUs => "Node Tree",
        }
    }

    pub const fn dbg_tree_empty(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "启动会话后点击「刷新节点树」查看舞台节点。",
            LocalePreference::EnUs => "Start a session and click Refresh Tree.",
        }
    }

    pub const fn dbg_props_and_eval(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "节点属性与表达式求值",
            LocalePreference::EnUs => "Node Properties & Eval",
        }
    }

    pub const fn dbg_eval_placeholder(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "输入 JS 表达式（例如 1 + 1）",
            LocalePreference::EnUs => "Evaluate JS (e.g. 1 + 1)",
        }
    }

    pub const fn dbg_eval_run(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "执行",
            LocalePreference::EnUs => "Eval",
        }
    }

    pub const fn dbg_eval_empty(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "尚未执行表达式。",
            LocalePreference::EnUs => "No evaluation result yet.",
        }
    }

    pub const fn dbg_props_empty(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "在左侧节点树中选择节点以查看属性 JSON。",
            LocalePreference::EnUs => "Select a node in the tree to inspect its properties.",
        }
    }

    pub const fn dbg_save_screenshot(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "保存图片",
            LocalePreference::EnUs => "Save",
        }
    }

    pub const fn dbg_engine_logs(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "引擎日志",
            LocalePreference::EnUs => "Engine Logs",
        }
    }

    pub fn dbg_log_level_label(&self, lvl: &str) -> &'static str {
        match (self.locale, lvl) {
            (LocalePreference::ZhCn, "ALL") => "全部",
            (LocalePreference::EnUs, "ALL") => "ALL",
            (_, "ERROR") => "ERROR",
            (_, "WARN") => "WARN",
            (_, "INFO") => "INFO",
            (_, "DEBUG") => "DEBUG",
            _ => "ALL",
        }
    }

    // Settings & Output
    pub const fn theme_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "主题",
            LocalePreference::EnUs => "Theme",
        }
    }

    pub const fn theme_name(&self, pref: ThemePreference) -> &'static str {
        match (self.locale, pref) {
            (LocalePreference::ZhCn, ThemePreference::System) => "跟随系统",
            (LocalePreference::EnUs, ThemePreference::System) => "System",
            (_, ThemePreference::Light) => "Light",
            (_, ThemePreference::Dark) => "Dark",
            (_, ThemePreference::SolarizedLight) => "Solarized Light",
            (_, ThemePreference::OneDark) => "One Dark",
        }
    }

    pub const fn language_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "界面语言",
            LocalePreference::EnUs => "Language",
        }
    }

    pub const fn locale_name(&self, loc: LocalePreference) -> &'static str {
        match loc {
            LocalePreference::ZhCn => "简体中文",
            LocalePreference::EnUs => "English",
        }
    }

    pub const fn tasks_and_output(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "任务与输出",
            LocalePreference::EnUs => "Tasks & Output",
        }
    }

    pub const fn task_output(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "任务输出",
            LocalePreference::EnUs => "Task Output",
        }
    }

    pub const fn logs_empty(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "暂无日志输出",
            LocalePreference::EnUs => "No log entries yet",
        }
    }

    pub const fn clear_logs(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "清空",
            LocalePreference::EnUs => "Clear",
        }
    }

    pub const fn ready_status(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "就绪",
            LocalePreference::EnUs => "Ready",
        }
    }
}
