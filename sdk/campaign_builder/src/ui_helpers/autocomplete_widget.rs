// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
//
// This file also incorporates code adapted from `egui_autocomplete` 12.0.0
// (https://github.com/JakeHandsome/egui_autocomplete), which has no release
// compatible with egui 0.35. Vendored here so campaign_builder can move to
// egui 0.35 without waiting on an upstream release.
//
// egui_autocomplete is MIT-licensed:
//
// Copyright (c) 2023 Jake Hansen
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A text edit with a fuzzy-matched autocomplete dropdown, vendored from
//! `egui_autocomplete` and ported to egui 0.35.
//!
//! Only the subset of the original crate's API actually used by
//! [`super::autocomplete`] is kept: `new`, `highlight_matches`, and
//! `max_suggestions`.

use eframe::egui;
use egui::text::LayoutJob;
use egui::{Context, FontId, Id, Key, Modifiers, Popup, PopupCloseBehavior, TextEdit, Widget};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::cmp::Reverse;

/// Used to set properties on the internal [`TextEdit`].
type SetTextEditProperties = dyn FnOnce(TextEdit) -> TextEdit;

/// An extension to [`egui::TextEdit`] that shows a fuzzy-matched autocomplete
/// dropdown while typing.
pub struct AutoCompleteTextEdit<'a, T> {
    /// Contents of text edit passed into [`egui::TextEdit`].
    text_field: &'a mut String,
    /// Data to use as the search term.
    search: T,
    /// A limit that can be placed on the maximum number of autocomplete suggestions shown.
    max_suggestions: usize,
    /// If true, highlights the matching indices in the dropdown.
    highlight: bool,
    /// Used to set properties on the internal `TextEdit`.
    set_properties: Option<Box<SetTextEditProperties>>,
}

impl<'a, T, S> AutoCompleteTextEdit<'a, T>
where
    T: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    /// Creates a new [`AutoCompleteTextEdit`].
    pub fn new(text_field: &'a mut String, search: T) -> Self {
        Self {
            text_field,
            search,
            max_suggestions: 10,
            highlight: false,
            set_properties: None,
        }
    }
}

impl<T, S> AutoCompleteTextEdit<'_, T>
where
    T: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    /// This determines the number of options that appear in the dropdown menu.
    pub fn max_suggestions(mut self, max_suggestions: usize) -> Self {
        self.max_suggestions = max_suggestions;
        self
    }

    /// If set to true, characters will be highlighted in the dropdown to show the match.
    pub fn highlight_matches(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Can be used to set properties on the internal [`egui::TextEdit`] (e.g. hint text).
    pub fn set_text_edit_properties(
        mut self,
        set_properties: impl FnOnce(TextEdit) -> TextEdit + 'static,
    ) -> Self {
        self.set_properties = Some(Box::new(set_properties));
        self
    }
}

impl<T, S> Widget for AutoCompleteTextEdit<'_, T>
where
    T: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    /// The response returned is the response from the internal text edit.
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self {
            text_field,
            search,
            max_suggestions,
            highlight,
            set_properties,
        } = self;

        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        let mut state = AutoCompleteTextEditState::load(ui.ctx(), id).unwrap_or_default();

        // Only consume up/down presses if the text box is focused. This overrides the
        // default behavior of moving to the start/end of the string.
        let up_pressed = state.focused
            && ui.input_mut(|input| input.consume_key(Modifiers::default(), Key::ArrowUp));
        let down_pressed = state.focused
            && ui.input_mut(|input| input.consume_key(Modifiers::default(), Key::ArrowDown));

        let mut text_edit = TextEdit::singleline(text_field);
        if let Some(set_properties) = set_properties {
            text_edit = set_properties(text_edit);
        }
        let text_edit_output = text_edit.show(ui);

        let mut text_response = text_edit_output.response.response;
        state.focused = text_response.has_focus();

        let matcher = SkimMatcherV2::default().ignore_case();

        let match_results = {
            let mut match_results = search
                .into_iter()
                .filter_map(|s| {
                    let score = matcher.fuzzy_indices(s.as_ref(), text_field.as_str());
                    score.map(|(score, indices)| (s, score, indices))
                })
                .collect::<Vec<_>>();
            match_results.sort_by_key(|k| Reverse(k.1));
            match_results
        };

        if text_response.changed()
            || (state.selected_index.is_some()
                && state.selected_index.unwrap() >= match_results.len())
        {
            state.selected_index = None;
        }

        state.update_index(
            down_pressed,
            up_pressed,
            match_results.len(),
            max_suggestions,
        );

        let popup = Popup::from_response(&text_response)
            .layout(egui::Layout::top_down_justified(egui::Align::LEFT))
            .close_behavior(PopupCloseBehavior::IgnoreClicks)
            .id(id)
            .align(egui::RectAlign::BOTTOM_START)
            .open(state.focused && !text_field.is_empty() && !match_results.is_empty());

        let accepted_by_keyboard = ui.input(|input| input.key_pressed(Key::Enter))
            || ui.input(|input| input.key_pressed(Key::Tab));
        if let (Some(index), true) = (
            state.selected_index,
            accepted_by_keyboard || !popup.is_open(),
        ) {
            let match_result = match_results[index].0.as_ref();
            text_field.replace_range(.., match_result);
            state.selected_index = None;
            text_response.mark_changed();
        }

        popup.show(|ui| {
            for (i, (output, _, match_indices)) in
                match_results.iter().take(max_suggestions).enumerate()
            {
                let mut selected = state.selected_index == Some(i);

                let text = if highlight {
                    highlight_matches(
                        output.as_ref(),
                        match_indices,
                        ui.style().visuals.widgets.active.text_color(),
                    )
                } else {
                    let mut job = LayoutJob::default();
                    job.append(output.as_ref(), 0.0, egui::TextFormat::default());
                    job
                };
                if ui.toggle_value(&mut selected, text).hovered() {
                    state.selected_index = Some(i);
                }
            }
        });

        state.store(ui.ctx(), id);

        text_response
    }
}

/// Highlights all the match indices in the provided text.
fn highlight_matches(text: &str, match_indices: &[usize], color: egui::Color32) -> LayoutJob {
    let mut formatted = LayoutJob::default();
    let mut it = text.char_indices().enumerate().peekable();
    while let Some((char_idx, (byte_idx, c))) = it.next() {
        let start = byte_idx;
        let mut end = byte_idx + (c.len_utf8() - 1);
        let match_state = match_indices.contains(&char_idx);
        while let Some((peek_char_idx, (_, k))) = it.peek() {
            if match_state == match_indices.contains(peek_char_idx) {
                end += k.len_utf8();
                _ = it.next();
            } else {
                break;
            }
        }
        let format = if match_state {
            egui::TextFormat::simple(FontId::default(), color)
        } else {
            egui::TextFormat::default()
        };
        let slice = &text[start..=end];
        formatted.append(slice, 0.0, format);
    }
    formatted
}

/// Stores the currently selected index in egui state.
#[derive(Debug, Clone, Default)]
struct AutoCompleteTextEditState {
    /// Currently selected index, is `None` if nothing is selected.
    selected_index: Option<usize>,
    /// Whether or not the text edit was focused last frame.
    focused: bool,
}

impl AutoCompleteTextEditState {
    /// Store the state with egui.
    fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }

    /// Get the state from egui if it exists.
    fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    /// Updates the selected index, keeping it in bounds.
    fn update_index(
        &mut self,
        down_pressed: bool,
        up_pressed: bool,
        match_results_count: usize,
        max_suggestions: usize,
    ) {
        self.selected_index = match self.selected_index {
            _ if match_results_count == 0 || max_suggestions == 0 => None,
            Some(index) if down_pressed => {
                if index + 1 < match_results_count.min(max_suggestions) {
                    Some(index + 1)
                } else {
                    None
                }
            }
            Some(index) if up_pressed => {
                if index == 0 {
                    None
                } else {
                    Some(index - 1)
                }
            }
            None if down_pressed => Some(0),
            None if up_pressed => Some(match_results_count.min(max_suggestions) - 1),
            Some(index) => Some(index),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increment_index() {
        let mut state = AutoCompleteTextEditState::default();
        assert_eq!(None, state.selected_index);
        state.update_index(false, false, 10, 10);
        assert_eq!(None, state.selected_index);
        state.update_index(true, false, 10, 10);
        assert_eq!(Some(0), state.selected_index);
        state.update_index(true, false, 2, 3);
        assert_eq!(Some(1), state.selected_index);
        state.update_index(true, false, 2, 3);
        assert_eq!(None, state.selected_index);
        state.update_index(true, false, 10, 3);
        assert_eq!(Some(0), state.selected_index);
        state.update_index(true, false, 10, 3);
        state.update_index(true, false, 10, 3);
        assert_eq!(Some(2), state.selected_index);
        state.update_index(true, false, 10, 3);
        assert_eq!(None, state.selected_index);
        state.update_index(false, true, 10, 3);
        assert_eq!(Some(2), state.selected_index);
    }

    #[test]
    fn decrement_index() {
        let mut state = AutoCompleteTextEditState {
            selected_index: Some(1),
            ..Default::default()
        };
        state.update_index(false, false, 10, 10);
        assert_eq!(Some(1), state.selected_index);
        state.update_index(false, true, 10, 10);
        assert_eq!(Some(0), state.selected_index);
        state.update_index(false, true, 10, 10);
        assert_eq!(None, state.selected_index);
    }

    #[test]
    fn highlight() {
        let text = String::from("Test123áéíó");
        let match_indices = vec![1, 5, 6, 8, 9, 10];
        let layout = highlight_matches(&text, &match_indices, egui::Color32::RED);
        assert_eq!(6, layout.sections.len());
        let sec1 = layout.sections.first().unwrap();
        assert_eq!(&text[sec1.byte_range.start.0..sec1.byte_range.end.0], "T");
        assert_ne!(sec1.format.color, egui::Color32::RED);

        let sec2 = layout.sections.get(1).unwrap();
        assert_eq!(&text[sec2.byte_range.start.0..sec2.byte_range.end.0], "e");
        assert_eq!(sec2.format.color, egui::Color32::RED);

        let sec3 = layout.sections.get(2).unwrap();
        assert_eq!(&text[sec3.byte_range.start.0..sec3.byte_range.end.0], "st1");
        assert_ne!(sec3.format.color, egui::Color32::RED);

        let sec4 = layout.sections.get(3).unwrap();
        assert_eq!(&text[sec4.byte_range.start.0..sec4.byte_range.end.0], "23");
        assert_eq!(sec4.format.color, egui::Color32::RED);

        let sec5 = layout.sections.get(4).unwrap();
        assert_eq!(&text[sec5.byte_range.start.0..sec5.byte_range.end.0], "á");
        assert_ne!(sec5.format.color, egui::Color32::RED);

        let sec6 = layout.sections.get(5).unwrap();
        assert_eq!(&text[sec6.byte_range.start.0..sec6.byte_range.end.0], "éíó");
        assert_eq!(sec6.format.color, egui::Color32::RED);
    }
}
