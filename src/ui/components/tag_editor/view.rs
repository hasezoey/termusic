/**
 * MIT License
 *
 * termusic - Copyright (C) 2021 Larry Hao
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
// use crate::config::Settings;
use crate::ui::components::{
    LabelGeneric, TECounterDelete, TEHelpPopup, TEInputArtist, TEInputTitle, TERadioTag,
    TESelectLyric, TETableLyricOptions, TETextareaLyric,
};
use crate::utils::{draw_area_in_relative, draw_area_top_right_absolute};

use crate::track::Track;
use crate::ui::model::Model;
use crate::ui::{Id, IdTagEditor};
use std::convert::TryFrom;
use std::path::Path;
use tuirealm::props::{Alignment, AttrValue, Attribute, PropPayload, PropValue, TextSpan};
use tuirealm::tui::layout::{Constraint, Direction, Layout};
use tuirealm::tui::widgets::Clear;
use tuirealm::State;

impl Model {
    #[allow(clippy::too_many_lines)]
    pub fn view_tag_editor(&mut self) {
        assert!(self
            .terminal
            .raw_mut()
            .draw(|f| {
                let select_lyric_len =
                    match self.app.state(&Id::TagEditor(IdTagEditor::SelectLyric)) {
                        Ok(State::One(_)) => 3,
                        _ => 8,
                    };
                if self.app.mounted(&Id::TagEditor(IdTagEditor::LabelHint)) {
                    f.render_widget(Clear, f.size());
                    let chunks_main = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(0)
                        .constraints(
                            [
                                Constraint::Length(1),
                                Constraint::Length(3),
                                Constraint::Length(3),
                                Constraint::Length(3),
                                Constraint::Min(2),
                                Constraint::Length(1),
                            ]
                            .as_ref(),
                        )
                        .split(f.size());

                    let chunks_middle1 = Layout::default()
                        .direction(Direction::Horizontal)
                        .margin(0)
                        .constraints(
                            [
                                Constraint::Ratio(1, 4),
                                Constraint::Ratio(2, 4),
                                Constraint::Ratio(1, 4),
                            ]
                            .as_ref(),
                        )
                        .split(chunks_main[1]);
                    let chunks_middle2 = Layout::default()
                        .direction(Direction::Horizontal)
                        .margin(0)
                        .constraints([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)].as_ref())
                        .split(chunks_main[4]);

                    let chunks_middle2_right = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(0)
                        .constraints(
                            [Constraint::Length(select_lyric_len), Constraint::Min(2)].as_ref(),
                        )
                        .split(chunks_middle2[1]);

                    let chunks_middle2_right_top = Layout::default()
                        .direction(Direction::Horizontal)
                        .margin(0)
                        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
                        .split(chunks_middle2_right[0]);

                    self.app
                        .view(&Id::TagEditor(IdTagEditor::LabelHint), f, chunks_main[0]);
                    self.app.view(&Id::Label, f, chunks_main[5]);
                    self.app.view(
                        &Id::TagEditor(IdTagEditor::InputArtist),
                        f,
                        chunks_middle1[0],
                    );
                    self.app.view(
                        &Id::TagEditor(IdTagEditor::InputTitle),
                        f,
                        chunks_middle1[1],
                    );
                    self.app
                        .view(&Id::TagEditor(IdTagEditor::RadioTag), f, chunks_middle1[2]);
                    self.app.view(
                        &Id::TagEditor(IdTagEditor::TableLyricOptions),
                        f,
                        chunks_middle2[0],
                    );
                    self.app.view(
                        &Id::TagEditor(IdTagEditor::SelectLyric),
                        f,
                        chunks_middle2_right_top[0],
                    );
                    self.app.view(
                        &Id::TagEditor(IdTagEditor::CounterDelete),
                        f,
                        chunks_middle2_right_top[1],
                    );
                    self.app.view(
                        &Id::TagEditor(IdTagEditor::TextareaLyric),
                        f,
                        chunks_middle2_right[1],
                    );

                    if self.app.mounted(&Id::TagEditor(IdTagEditor::HelpPopup)) {
                        let popup = draw_area_in_relative(f.size(), 50, 70);
                        f.render_widget(Clear, popup);
                        self.app
                            .view(&Id::TagEditor(IdTagEditor::HelpPopup), f, popup);
                    }
                    if self.app.mounted(&Id::MessagePopup) {
                        let popup = draw_area_top_right_absolute(f.size(), 25, 4);
                        f.render_widget(Clear, popup);
                        self.app.view(&Id::MessagePopup, f, popup);
                    }
                    if self.app.mounted(&Id::ErrorPopup) {
                        let popup = draw_area_in_relative(f.size(), 50, 4);
                        f.render_widget(Clear, popup);
                        self.app.view(&Id::ErrorPopup, f, popup);
                    }
                }
            })
            .is_ok());
    }

    pub fn mount_tageditor(&mut self, node_id: &str) {
        let p: &Path = Path::new(node_id);
        if p.is_dir() {
            self.mount_error_popup("directory doesn't have tag!");
            return;
        }

        let p = p.to_string_lossy();
        match Track::read_from_path(p.as_ref(), false) {
            Ok(s) => {
                assert!(self
                    .app
                    .remount(
                        Id::TagEditor(IdTagEditor::LabelHint),
                        Box::new(LabelGeneric::new(&self.config, "Press <ENTER> to search:")),
                        vec![]
                    )
                    .is_ok());
                assert!(self
                    .app
                    .remount(
                        Id::TagEditor(IdTagEditor::InputArtist),
                        Box::new(TEInputArtist::default()),
                        vec![]
                    )
                    .is_ok());
                assert!(self
                    .app
                    .remount(
                        Id::TagEditor(IdTagEditor::InputTitle),
                        Box::new(TEInputTitle::default()),
                        vec![]
                    )
                    .is_ok());
                assert!(self
                    .app
                    .remount(
                        Id::TagEditor(IdTagEditor::RadioTag),
                        Box::new(TERadioTag::default()),
                        vec![]
                    )
                    .is_ok());
                assert!(self
                    .app
                    .remount(
                        Id::TagEditor(IdTagEditor::TableLyricOptions),
                        Box::new(TETableLyricOptions::default()),
                        vec![]
                    )
                    .is_ok());
                assert!(self
                    .app
                    .remount(
                        Id::TagEditor(IdTagEditor::SelectLyric),
                        Box::new(TESelectLyric::default()),
                        vec![]
                    )
                    .is_ok());
                assert!(self
                    .app
                    .remount(
                        Id::TagEditor(IdTagEditor::CounterDelete),
                        Box::new(TECounterDelete::new(5)),
                        vec![]
                    )
                    .is_ok());
                assert!(self
                    .app
                    .remount(
                        Id::TagEditor(IdTagEditor::TextareaLyric),
                        Box::new(TETextareaLyric::default()),
                        vec![]
                    )
                    .is_ok());

                self.app
                    .active(&Id::TagEditor(IdTagEditor::InputArtist))
                    .ok();
                self.init_by_song(&s);
            }
            Err(e) => {
                self.mount_error_popup(format!("song load error: {}", e).as_ref());
            }
        };
        if let Err(e) = self.update_photo() {
            self.mount_error_popup(format!("clear photo error: {}", e).as_str());
        }
    }
    pub fn umount_tageditor(&mut self) {
        self.app.umount(&Id::TagEditor(IdTagEditor::LabelHint)).ok();
        self.app
            .umount(&Id::TagEditor(IdTagEditor::InputArtist))
            .ok();
        self.app
            .umount(&Id::TagEditor(IdTagEditor::InputTitle))
            .ok();
        self.app.umount(&Id::TagEditor(IdTagEditor::RadioTag)).ok();
        self.app
            .umount(&Id::TagEditor(IdTagEditor::TableLyricOptions))
            .ok();
        self.app
            .umount(&Id::TagEditor(IdTagEditor::SelectLyric))
            .ok();
        self.app
            .umount(&Id::TagEditor(IdTagEditor::CounterDelete))
            .ok();
        self.app
            .umount(&Id::TagEditor(IdTagEditor::TextareaLyric))
            .ok();
        if let Err(e) = self.update_photo() {
            self.mount_error_popup(format!("update photo error: {}", e).as_ref());
        }
        self.global_fix_focus();
    }
    // initialize the value in tageditor based on info from Song
    pub fn init_by_song(&mut self, s: &Track) {
        self.tageditor_song = Some(s.clone());
        if let Some(artist) = s.artist() {
            assert!(self
                .app
                .attr(
                    &Id::TagEditor(IdTagEditor::InputArtist),
                    Attribute::Value,
                    AttrValue::String(artist.to_string()),
                )
                .is_ok());
        }

        if let Some(title) = s.title() {
            assert!(self
                .app
                .attr(
                    &Id::TagEditor(IdTagEditor::InputTitle),
                    Attribute::Value,
                    AttrValue::String(title.to_string()),
                )
                .is_ok());
        }

        if s.lyric_frames_is_empty() {
            self.init_by_song_no_lyric();
            return;
        }

        let mut vec_lang: Vec<String> = vec![];
        if let Some(lf) = s.lyric_frames() {
            for l in lf {
                vec_lang.push(l.description.clone());
            }
        }
        vec_lang.sort();

        assert!(self
            .app
            .attr(
                &Id::TagEditor(IdTagEditor::SelectLyric),
                Attribute::Content,
                AttrValue::Payload(PropPayload::Vec(
                    vec_lang
                        .iter()
                        .map(|x| PropValue::Str((*x).to_string()))
                        .collect(),
                )),
            )
            .is_ok());
        if let Ok(vec_lang_len_isize) = isize::try_from(vec_lang.len()) {
            assert!(self
                .app
                .attr(
                    &Id::TagEditor(IdTagEditor::CounterDelete),
                    Attribute::Value,
                    AttrValue::Number(vec_lang_len_isize),
                )
                .is_ok());
        }
        let mut vec_lyric: Vec<TextSpan> = vec![];
        if let Some(f) = s.lyric_selected() {
            for line in f.text.split('\n') {
                vec_lyric.push(TextSpan::from(line));
            }
        }
        assert!(self
            .app
            .attr(
                &Id::TagEditor(IdTagEditor::TextareaLyric),
                Attribute::Title,
                AttrValue::Title((
                    format!("{} Lyrics", vec_lang[s.lyric_selected_index()]),
                    Alignment::Left
                ))
            )
            .is_ok());

        assert!(self
            .app
            .attr(
                &Id::TagEditor(IdTagEditor::TextareaLyric),
                Attribute::Text,
                AttrValue::Payload(PropPayload::Vec(
                    vec_lyric.iter().cloned().map(PropValue::TextSpan).collect()
                ))
            )
            .is_ok());
    }

    fn init_by_song_no_lyric(&mut self) {
        assert!(self
            .app
            .attr(
                &Id::TagEditor(IdTagEditor::SelectLyric),
                Attribute::Content,
                AttrValue::Payload(PropPayload::Vec(
                    ["Empty"]
                        .iter()
                        .map(|x| PropValue::Str((*x).to_string()))
                        .collect(),
                )),
            )
            .is_ok());
        assert!(self
            .app
            .attr(
                &Id::TagEditor(IdTagEditor::CounterDelete),
                Attribute::Value,
                AttrValue::Number(0),
            )
            .is_ok());

        assert!(self
            .app
            .attr(
                &Id::TagEditor(IdTagEditor::TextareaLyric),
                Attribute::Title,
                AttrValue::Title(("Empty Lyric".to_string(), Alignment::Left))
            )
            .is_ok());
        assert!(self
            .app
            .attr(
                &Id::TagEditor(IdTagEditor::TextareaLyric),
                Attribute::Text,
                AttrValue::Payload(PropPayload::Vec(vec![PropValue::TextSpan(TextSpan::from(
                    "No Lyrics."
                )),]))
            )
            .is_ok());
    }

    pub fn mount_tageditor_help(&mut self) {
        assert!(self
            .app
            .remount(
                Id::TagEditor(IdTagEditor::HelpPopup),
                Box::new(TEHelpPopup::default()),
                vec![]
            )
            .is_ok());
        // Active help
        assert!(self
            .app
            .active(&Id::TagEditor(IdTagEditor::HelpPopup))
            .is_ok());
    }
}
