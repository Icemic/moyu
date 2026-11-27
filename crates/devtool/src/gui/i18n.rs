//! Lightweight internationalization strings (ZhCn default, EnUs reserved).

use crate::config::LocalePreference;

#[derive(Debug, Clone, Copy)]
pub struct I18n {
    pub locale: LocalePreference,
}

impl I18n {
    pub const fn new(locale: LocalePreference) -> Self {
        Self { locale }
    }

    pub const fn app_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "末语开发工具 (Moyu Devtool)",
            LocalePreference::EnUs => "Moyu Devtool",
        }
    }

    pub const fn nav_project(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "工程总览",
            LocalePreference::EnUs => "Project",
        }
    }

    pub const fn nav_engine(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "引擎管理",
            LocalePreference::EnUs => "Engine",
        }
    }

    pub const fn nav_run_pack(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "运行与打包",
            LocalePreference::EnUs => "Run & Pack",
        }
    }

    pub const fn nav_debug(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "调试器",
            LocalePreference::EnUs => "Debugger",
        }
    }

    pub const fn nav_settings(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "环境与设置",
            LocalePreference::EnUs => "Settings",
        }
    }

    pub const fn open_project(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "打开工程目录",
            LocalePreference::EnUs => "Open Project",
        }
    }

    pub const fn no_project_selected(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "未打开工程",
            LocalePreference::EnUs => "No project opened",
        }
    }

    pub const fn recent_projects(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "最近工程",
            LocalePreference::EnUs => "Recent Projects",
        }
    }

    pub const fn create_project_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "新建最小工程骨架",
            LocalePreference::EnUs => "Create New Project",
        }
    }

    pub const fn project_name_placeholder(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "输入工程名（如 my-visual-novel）",
            LocalePreference::EnUs => "Project name (e.g. my-visual-novel)",
        }
    }

    pub const fn pick_parent_and_create(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "选择父目录并创建",
            LocalePreference::EnUs => "Pick Folder & Create",
        }
    }

    pub const fn recheck_assets(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "重新检查多分辨率资源",
            LocalePreference::EnUs => "Recheck Multi-Res Assets",
        }
    }

    pub const fn fetch_versions(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "拉取远端版本清单",
            LocalePreference::EnUs => "Fetch Manifest",
        }
    }

    pub const fn update_latest(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "一键更新当前渠道最新版",
            LocalePreference::EnUs => "Update to Latest",
        }
    }

    pub const fn download_selected(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "下载所选平台",
            LocalePreference::EnUs => "Download Selected",
        }
    }

    pub const fn local_versions(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "本地已下载引擎版本",
            LocalePreference::EnUs => "Local Engine Versions",
        }
    }

    pub const fn activate(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "设为激活",
            LocalePreference::EnUs => "Activate",
        }
    }

    pub const fn active_badge(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "当前激活",
            LocalePreference::EnUs => "Active",
        }
    }

    pub const fn run_native(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "启动原生运行",
            LocalePreference::EnUs => "Run Native",
        }
    }

    pub const fn run_web(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "启动 Web 服务 (6320)",
            LocalePreference::EnUs => "Run Web (6320)",
        }
    }

    pub const fn stop_task(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "停止 / 取消",
            LocalePreference::EnUs => "Stop / Cancel",
        }
    }

    pub const fn start_pack(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "开始打包",
            LocalePreference::EnUs => "Start Pack",
        }
    }

    pub const fn gen_commands_schema(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "生成 commands.schema.json",
            LocalePreference::EnUs => "Generate commands.schema.json",
        }
    }

    pub const fn gen_ui_schema(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "生成 ui.schema.json",
            LocalePreference::EnUs => "Generate ui.schema.json",
        }
    }

    pub const fn android_password_placeholder(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Android Release 签名密码（debug-apk 留空）",
            LocalePreference::EnUs => "Android release signing password",
        }
    }

    pub const fn env_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "Node.js 与包管理器环境",
            LocalePreference::EnUs => "Node.js & Package Managers",
        }
    }

    pub const fn redetect_env(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "重新检测环境",
            LocalePreference::EnUs => "Re-detect",
        }
    }

    pub const fn install_managed_node(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "一键安装受管 Node LTS",
            LocalePreference::EnUs => "Install Managed Node LTS",
        }
    }

    pub const fn theme_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "主题外观",
            LocalePreference::EnUs => "Appearance",
        }
    }

    pub const fn theme_light(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "浅色",
            LocalePreference::EnUs => "Light",
        }
    }

    pub const fn theme_dark(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "深色",
            LocalePreference::EnUs => "Dark",
        }
    }

    pub const fn theme_system(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "跟随系统",
            LocalePreference::EnUs => "System",
        }
    }

    pub const fn language_label(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "界面语言",
            LocalePreference::EnUs => "Language",
        }
    }

    pub const fn logs_title(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "实时日志输出",
            LocalePreference::EnUs => "Output Logs",
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
}
