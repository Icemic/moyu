//! Virtualized scrolling log list backed by `gpui::uniform_list`.

use gpui::{App, Context, Window, div, prelude::*, px, uniform_list};

use crate::gui::theme::Theme;
use crate::model::tasks::{LogLevel, TaskLogEntry};

pub struct LogListView {
    pub entries: Vec<TaskLogEntry>,
    pub empty_label: String,
}

impl LogListView {
    pub fn new(empty_label: impl Into<String>) -> Self {
        Self {
            entries: Vec::new(),
            empty_label: empty_label.into(),
        }
    }

    #[allow(dead_code)]
    pub fn push(&mut self, entry: TaskLogEntry, cx: &mut Context<Self>) {
        if self.entries.len() >= 2000 {
            self.entries.remove(0);
        }
        self.entries.push(entry);
        cx.notify();
    }

    pub fn set_empty_label(&mut self, label: impl Into<String>, cx: &mut Context<Self>) {
        self.empty_label = label.into();
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.entries.clear();
        cx.notify();
    }
}

impl Render for LogListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        if self.entries.is_empty() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_tertiary)
                .text_size(px(12.0))
                .child(self.empty_label.clone())
                .into_any_element();
        }

        let count = self.entries.len();
        uniform_list(
            "devtool-log-list",
            count,
            cx.processor(move |this, range: std::ops::Range<usize>, _window, cx: &mut Context<Self>| {
                let theme = Theme::global(cx);
                let mut rows = Vec::with_capacity(range.len());
                for ix in range {
                    let entry = &this.entries[ix];
                    let level_color = match entry.level {
                        LogLevel::Info => theme.text_secondary,
                        LogLevel::Success => theme.success,
                        LogLevel::Warn => theme.warn,
                        LogLevel::Error => theme.danger,
                    };
                    rows.push(
                        div()
                            .id(ix)
                            .px(px(8.0))
                            .py(px(2.0))
                            .flex()
                            .flex_row()
                            .gap(px(8.0))
                            .text_size(px(12.0))
                            .child(
                                div()
                                    .text_color(level_color)
                                    .child(format!("[{}]", entry.level.prefix())),
                            )
                            .child(div().text_color(theme.text).child(entry.line.clone())),
                    );
                }
                rows
            }),
        )
        .size_full()
        .into_any_element()
    }
}

#[allow(dead_code)]
pub fn bind_log_theme(_cx: &App) {}
