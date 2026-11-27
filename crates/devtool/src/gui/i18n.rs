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

    pub const fn search_placeholder(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "输入文本测试中文 IME 组合输入...",
            LocalePreference::EnUs => "Type here to test input...",
        }
    }

    pub const fn stage_placeholder_hint(&self) -> &'static str {
        match self.locale {
            LocalePreference::ZhCn => "阶段 3.1 基础架构与组件库已就绪，阶段 3.2 将接入完整业务页面。",
            LocalePreference::EnUs => "Phase 3.1 shell & components ready; Phase 3.2 wires full views.",
        }
    }
}
