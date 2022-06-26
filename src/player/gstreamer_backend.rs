/**
 * MIT License
 *
 * termusic - Copyright (c) 2021 Larry Hao
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
use super::{PlayerMsg, PlayerTrait};
use crate::config::Termusic;
use anyhow::{anyhow, bail, Result};
// use fragile::Fragile;
use gst::ClockTime;
use gstreamer as gst;
use gstreamer::prelude::*;
use std::cmp;
// use std::rc::Rc;
// use glib::{FlagsClass, MainContext};
use std::sync::mpsc::Sender;

use glib::{FlagsClass, MainContext};
use gst::Element;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct GStreamer {
    playbin: Element,
    next_uri: Arc<Mutex<Option<String>>>,
    main_tx: glib::Sender<PlayerMsg>,
    paused: bool,
    volume: i32,
    speed: f32,
    pub gapless: bool,
    tx: Sender<PlayerMsg>,
}

impl GStreamer {
    pub fn new(config: &Termusic, message_tx: Sender<PlayerMsg>) -> Self {
        gst::init().expect("Couldn't initialize Gstreamer");

        let ctx = glib::MainContext::default();
        let _guard = ctx.acquire();
        let mainloop = glib::MainLoop::new(Some(&ctx), false);

        let playbin = gst::ElementFactory::make("playbin", None)
            .expect("Unable to create the `playbin` element");

        // Set flags to show Audio and Video but ignore Subtitles
        let flags = playbin.property_value("flags");
        let flags_class = FlagsClass::new(flags.type_()).unwrap();

        let flags = flags_class
            .builder_with_value(flags)
            .unwrap()
            .set_by_nick("audio")
            .set_by_nick("video")
            .unset_by_nick("text")
            .build()
            .unwrap();
        playbin.set_property_from_value("flags", &flags);

        // Asynchronous channel to communicate with main() with
        let (main_tx, main_rx) = MainContext::channel(glib::Priority::default());
        // Handle messages from GSTreamer bus
        playbin
            .bus()
            .expect("Failed to get GStreamer message bus")
            .add_watch(glib::clone!(@strong main_tx => move |_bus, msg| {
                match msg.view() {
                    gst::MessageView::Eos(_) =>
                        main_tx.send(PlayerMsg::Eos)
                        .expect("Unable to send message to main()"),
                    gst::MessageView::Error(e) =>
                        glib::g_debug!("song", "{}", e.error()),
                        _ => (),
                }
                glib::Continue(true)
            }))
            .expect("Failed to connect to GStreamer message bus");

        let tx = message_tx.clone();
        main_rx.attach(
            None,
            glib::clone!(@strong mainloop => move |msg| {
                // match msg {
                    // BackendMessage::ReachedEndOfSong => {
                    //     // Backend switches to the next track itself,
                    //     // we just need to notify playlist about the change.
                    //     playlist.lockk().next();
                    // }
                    // BackendMessage::ReachedEndOfPlaylist => {
                    //     output.cleanup();
                    //     mainloop.quit();
                    // }
                    // BackendMessage::RequestNextSong => {
                    //     backend.enqueue(playlist.lockk().peek());
                    // }
                    // BackendMessage::State(state) => {
                    //     output.refresh(state, &playlist.lockk())
                    //         .ok(); // ignore any output errors
                    // }
                    // PlayerMsg::Eos => {
                        tx.send(msg).unwrap();
                    // }
                // };
                glib::Continue(true)
            }),
        );
        // mainloop.run();

        let volume = config.volume;
        let speed = config.speed;

        let this = Self {
            playbin,
            next_uri: Arc::new(Mutex::new(None)),
            main_tx,
            paused: false,
            volume,
            speed,
            gapless: true,
            tx: message_tx.clone(),
        };

        // Switch to next song when reaching end of current track
        // let tx = Fragile::new(message_tx.clone());
        // this.playbin.connect(
        //     "about-to-finish",
        //     false,
        //     glib::clone!(@strong this => move |_args| {
        //        this.dequeue();
        //        // tx.get().send(PlayerMsg::AboutToFinish).unwrap();
        //        None
        //     }),
        // );

        this
    }
    pub fn skip_one(&mut self) {
        self.tx.send(PlayerMsg::Eos).unwrap();
    }
    pub fn enqueue_next(&mut self, next_track: &str) {
        self.playbin
            .set_property("uri", Some(&format!("file:///{}", next_track)));
    }
    pub fn play(&mut self) {}

    /// Sets the song to be played after the end of the current one
    /// is reached. This is necessary for gapless playback.
    pub fn enqueue(&mut self, track: &str) {
        *self.next_uri.lock().unwrap() = Some(track.to_string());
    }

    /// Sets the playbin URI to `self.next_uri`, when it is not None.
    /// This function is to be used from GStreamer playbin's
    /// about-to-finish callback only.
    pub fn dequeue(&self) {
        if let Some(uri) = &*self.next_uri.lock().unwrap() {
            self.playbin.set_property("uri", uri);
            self.main_tx
                .send(PlayerMsg::Eos)
                .expect("Unable to send message to main()");
        }
    }
}

impl PlayerTrait for GStreamer {
    fn add_and_play(&mut self, song_str: &str) {
        self.playbin
            .set_state(gst::State::Ready)
            .expect("set gst state ready error.");
        self.playbin
            .set_property("uri", Some(&format!("file:///{}", song_str)));
        self.playbin
            .set_state(gst::State::Playing)
            .expect("set gst state playing error");
        // self.main_tx
        //     .send(PlayerMsg::Eos)
        //     .expect("Unable to send message to main()");
        // self.player.set_uri(Some(&format!("file:///{}", song_str)));
        // self.paused = false;

        self.play();
    }

    fn volume_up(&mut self) {
        self.volume = cmp::min(self.volume + 5, 100);
        // self.player.set_volume(f64::from(self.volume) / 100.0);
    }

    fn volume_down(&mut self) {
        self.volume = cmp::max(self.volume - 5, 0);
        // self.player.set_volume(f64::from(self.volume) / 100.0);
    }

    fn volume(&self) -> i32 {
        self.volume
    }

    fn set_volume(&mut self, mut volume: i32) {
        if volume > 100 {
            volume = 100;
        } else if volume < 0 {
            volume = 0;
        }
        self.volume = volume;
        // self.player.set_volume(f64::from(volume) / 100.0);
    }

    fn pause(&mut self) {
        self.paused = true;
        // self.player.pause();
        match self.playbin.current_state() {
            _ => self
                .playbin
                .set_state(gst::State::Paused)
                .expect("set gst state paused error"),
        };
    }

    fn resume(&mut self) {
        self.paused = false;
        // self.player.play();
        match self.playbin.current_state() {
            _ => self
                .playbin
                .set_state(gst::State::Playing)
                .expect("set gst state playing error in resume"),
        };
    }

    fn is_paused(&self) -> bool {
        self.playbin.current_state() == gst::State::Paused
    }

    #[allow(clippy::cast_sign_loss)]
    fn seek(&mut self, secs: i64) -> Result<()> {
        if let Ok((_, time_pos, duration)) = self.get_progress() {
            let mut seek_pos = time_pos + secs;
            if seek_pos < 0 {
                seek_pos = 0;
            }

            if seek_pos.cmp(&duration) == std::cmp::Ordering::Greater {
                bail! {"exceed max length"};
            }
            let seek_pos = ClockTime::from_seconds(seek_pos as u64);
            self.playbin
                .seek_simple(gst::SeekFlags::FLUSH, seek_pos)
                .ok(); // ignore any errors
        }
        Ok(())
    }

    #[allow(clippy::cast_precision_loss)]
    fn get_progress(&mut self) -> Result<(f64, i64, i64)> {
        let time_pos = match self
            .playbin
            .query_position::<gst::ClockTime>()
            .unwrap_or_default()
            .into()
        {
            Some(t) => ClockTime::seconds(t).try_into().unwrap_or(0),
            None => 0_i64,
        };
        let duration = match self
            .playbin
            .query_duration::<ClockTime>()
            .unwrap_or_default()
            .into()
        {
            Some(d) => ClockTime::seconds(d).try_into().unwrap_or(0),
            None => 0_i64,
        };
        let mut percent = (time_pos * 100)
            .checked_div(duration)
            .ok_or_else(|| anyhow!("divide error"))?;
        if percent > 100 {
            percent = 100;
        }
        Ok((percent as f64, time_pos, duration))
    }

    fn speed(&self) -> f32 {
        self.speed
    }

    fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
        // self.player.set_rate(speed.into());
    }

    fn speed_up(&mut self) {
        let mut speed = self.speed + 0.1;
        if speed > 3.0 {
            speed = 3.0;
        }
        self.set_speed(speed);
    }

    fn speed_down(&mut self) {
        let mut speed = self.speed - 0.1;
        if speed < 0.1 {
            speed = 0.1;
        }
        self.set_speed(speed);
    }
    fn stop(&mut self) {
        self.playbin.set_state(gst::State::Null).ok();
    }
}

impl Drop for GStreamer {
    /// Cleans up GStreamer pipeline when `Backend` is dropped.
    fn drop(&mut self) {
        self.playbin
            .set_state(gst::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");
    }
}
