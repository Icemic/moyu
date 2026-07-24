use anyhow::Result;
use moyu_core::base::Rect;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Command, EditableTarget, Focusable, Node, NodeBaseTrait, NodeEventSource};
use moyu_core::utils::convert::{JSValue, from_js, to_js};
use moyu_core::utils::layout::measure_children_layout_size;
use moyu_core::utils::patch::Patch;
use moyu_core::winit::event::{Ime, KeyEvent};
use moyu_core::winit::keyboard::{Key, ModifiersState, NamedKey};
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use unicode_segmentation::UnicodeSegmentation;

use crate::events::EditableEvent;

#[derive(Debug, Default, Node)]
pub struct Editable {
    pub value: String,
    composition: Option<String>,
    pub disabled: bool,
    pub read_only: bool,
    caret_rect: Option<Rect>,

    #[base]
    node_base: NodeBase,
}

impl Editable {
    pub fn new(label: String) -> Self {
        Self {
            node_base: NodeBase::new(label),
            ..Default::default()
        }
    }

    fn state(&self) -> EditableState {
        let composition_text = self.composition.as_ref().cloned().unwrap_or_default();

        EditableState {
            value: self.value.clone(),
            is_composing: self.composition.is_some(),
            composition_text,
        }
    }

    fn set_value(&mut self, value: String) {
        self.value = sanitize_single_line(&value);
        self.cancel_composition();
    }

    fn insert_text(&mut self, text: &str) {
        if self.disabled || self.read_only {
            return;
        }

        let text = sanitize_single_line(text);
        if text.is_empty() {
            return;
        }

        self.cancel_composition();
        self.value.push_str(&text);
        let state = self.state();
        self.send_event(EditableEvent::Change {
            state: state.clone(),
            source: EditableChangeSource::InsertText,
        });
        self.send_event(EditableEvent::Input { state });
    }

    fn delete_backward(&mut self) {
        if self.disabled || self.read_only {
            return;
        }

        self.cancel_composition();
        let Some((index, _)) = self.value.grapheme_indices(true).next_back() else {
            return;
        };

        self.value.truncate(index);
        let state = self.state();
        self.send_event(EditableEvent::Change {
            state: state.clone(),
            source: EditableChangeSource::DeleteBackward,
        });
        self.send_event(EditableEvent::Input { state });
    }

    fn update_preedit(&mut self, text: &str) {
        if self.disabled || self.read_only {
            return;
        }

        let text = sanitize_single_line(text);
        match self.composition.as_mut() {
            Some(composition_text) => {
                if *composition_text == text {
                    return;
                }

                *composition_text = text;
                let state = self.state();
                self.send_event(EditableEvent::CompositionUpdate {
                    state: state.clone(),
                });
                self.send_event(EditableEvent::Input { state });
            }
            None if text.is_empty() => {}
            None => {
                self.composition = Some(text);
                let state = self.state();
                self.send_event(EditableEvent::CompositionStart {
                    state: state.clone(),
                });
                self.send_event(EditableEvent::Input { state });
            }
        }
    }

    fn commit_composition(&mut self, text: &str) {
        if self.disabled || self.read_only {
            return;
        }

        let text = sanitize_single_line(text);
        let was_composing = self.composition.take().is_some();
        if !was_composing && text.is_empty() {
            return;
        }

        if !text.is_empty() {
            self.value.push_str(&text);
        }
        let state = self.state();

        if was_composing {
            self.send_event(EditableEvent::CompositionEnd {
                state: state.clone(),
            });
        }
        if !text.is_empty() {
            self.send_event(EditableEvent::Change {
                state: state.clone(),
                source: EditableChangeSource::InsertCompositionText,
            });
        }
        self.send_event(EditableEvent::Input { state });
    }

    fn settle_pending_clear(&mut self) {
        if self.composition.as_ref().is_some_and(String::is_empty) {
            self.cancel_composition();
        }
    }

    fn set_disabled(&mut self, disabled: bool) {
        if self.disabled == disabled {
            return;
        }

        self.disabled = disabled;
        if disabled {
            self.cancel_composition();
        }
    }

    fn set_read_only(&mut self, read_only: bool) {
        if self.read_only == read_only {
            return;
        }

        self.read_only = read_only;
        if read_only {
            self.cancel_composition();
        }
    }

    fn cancel_composition(&mut self) {
        if self.composition.take().is_none() {
            return;
        }

        let state = self.state();
        self.send_event(EditableEvent::CompositionEnd {
            state: state.clone(),
        });
        self.send_event(EditableEvent::Input { state });
    }
}

fn sanitize_single_line(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut in_line_break = false;

    for character in value.chars() {
        if matches!(character, '\r' | '\n') {
            if !in_line_break {
                result.push(' ');
                in_line_break = true;
            }
        } else {
            result.push(character);
            in_line_break = false;
        }
    }

    result
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct EditableProps {
    pub value: Patch<String>,
    pub disabled: Patch<bool>,
    pub read_only: Patch<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditableState {
    pub value: String,
    pub is_composing: bool,
    pub composition_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EditableChangeSource {
    InsertText,
    InsertCompositionText,
    DeleteBackward,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "subCommand")]
#[ts(export, optional_fields)]
pub enum EditableCommand {
    GetState,
    SetValue {
        value: String,
    },
    SetCaretRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}

impl Node for Editable {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        Ok(Box::new(Self::new(label.unwrap_or_default())))
    }

    fn node_type(&self) -> &'static str {
        "editable"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: EditableProps = match from_js(props) {
            Ok(props) => props,
            Err(error) => {
                log::error!("Error parsing editable props: {error:?}");
                return;
            }
        };

        match props.value {
            Patch::Set(value) => self.set_value(value),
            Patch::Reset => self.set_value(String::new()),
            Patch::Missing => {}
        }

        match props.disabled {
            Patch::Set(disabled) => self.set_disabled(disabled),
            Patch::Reset => self.set_disabled(false),
            Patch::Missing => {}
        }

        match props.read_only {
            Patch::Set(read_only) => self.set_read_only(read_only),
            Patch::Reset => self.set_read_only(false),
            Patch::Missing => {}
        }

        self.base_mut().pend_update();
    }

    fn measure(&mut self) {
        let (width, height) = measure_children_layout_size(self.base());
        self.base_mut().set_layout_size(width, height);
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }

    fn as_editable_target(&self) -> Option<&dyn EditableTarget> {
        Some(self)
    }

    fn as_editable_target_mut(&mut self) -> Option<&mut dyn EditableTarget> {
        Some(self)
    }

    fn as_command(&mut self) -> Option<&mut dyn Command> {
        Some(self)
    }
}

impl Focusable for Editable {}

impl EditableTarget for Editable {
    fn is_disabled(&self) -> bool {
        self.disabled
    }

    fn is_read_only(&self) -> bool {
        self.read_only
    }

    fn did_focus(&mut self) {
        self.send_event(EditableEvent::Focus {
            state: self.state(),
        });
    }

    fn did_blur(&mut self) {
        self.send_event(EditableEvent::Blur {
            state: self.state(),
        });
    }

    fn handle_keyboard_input(&mut self, event: &KeyEvent, modifiers: ModifiersState) {
        if !event.state.is_pressed() || self.disabled || self.read_only {
            return;
        }

        self.settle_pending_clear();
        match event.logical_key.as_ref() {
            Key::Named(NamedKey::Backspace) => self.delete_backward(),
            Key::Named(NamedKey::Space)
                if !modifiers.super_key() && (!modifiers.control_key() || modifiers.alt_key()) =>
            {
                self.insert_text(" ");
            }
            Key::Character(_)
                if !modifiers.super_key() && (!modifiers.control_key() || modifiers.alt_key()) =>
            {
                if let Some(text) = event.text.as_deref() {
                    self.insert_text(text);
                }
            }
            _ => {}
        }
    }

    fn handle_ime(&mut self, event: &Ime) {
        match event {
            Ime::Preedit(text, _) => self.update_preedit(text),
            Ime::Commit(text) => self.commit_composition(text),
            Ime::Enabled | Ime::Disabled => {}
        }
    }

    fn settle_pending_clear(&mut self) {
        self.settle_pending_clear();
    }

    fn cancel_composition(&mut self) {
        self.cancel_composition();
    }

    fn ime_cursor_rect(&self) -> Option<Rect> {
        self.caret_rect
    }
}

impl NodeEventSource for Editable {
    type Event = EditableEvent;
}

impl Command for Editable {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        match from_js(payload)? {
            EditableCommand::GetState => Ok(Some(to_js(&self.state())?)),
            EditableCommand::SetValue { value } => {
                self.set_value(value);
                Ok(None)
            }
            EditableCommand::SetCaretRect {
                x,
                y,
                width,
                height,
            } => {
                self.caret_rect = Some(Rect::new(x, y, width, height));
                Ok(None)
            }
        }
    }
}
