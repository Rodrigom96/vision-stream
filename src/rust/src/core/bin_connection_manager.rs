use std::sync::{Arc, Mutex};
use std::time::Instant;

use gst::prelude::*;

struct Context {
    pub reconnecting: bool,
    pub last_buffer_update: std::time::Instant,
    pub last_reconnect_time: std::time::Instant,
    pub async_state_watch_timeout: Option<glib::SourceId>,
    pub source_watch_timeout: Option<glib::SourceId>,
}

impl Drop for Context {
    fn drop(&mut self) {
        if let Some(source_id) = self.source_watch_timeout.take() {
            source_id.remove();
        };
        if let Some(source_id) = self.async_state_watch_timeout.take() {
            source_id.remove();
        }

        log::debug!("Drop bin connection manager context")
    }
}

fn watch_source_async_state_change(bin: &gst::Bin, ctx: &Arc<Mutex<Context>>) -> bool {
    let (ret, state, _pending) = bin.state(gst::ClockTime::ZERO);

    // Bin is still changing state ASYNC. Wait for some more time.
    if let Ok(success) = ret {
        if success == gst::StateChangeSuccess::Async {
            return true;
        }
    }

    // Bin state change failed / failed to get state
    if ret.is_err() {
        log::error!("Bin {} state change failed", bin.name());
        let mut ctx = ctx.lock().unwrap();
        ctx.reconnecting = true;
        ctx.async_state_watch_timeout = None;
        return false;
    }

    // Bin successfully changed state to PLAYING. Stop watching state
    if state == gst::State::Playing {
        let mut ctx = ctx.lock().unwrap();
        //ctx.reconecting = false;
        ctx.reconnecting = false;
        ctx.async_state_watch_timeout = None;
        return false;
    }

    // Bin has stopped ASYNC state change but has not gone into
    // PLAYING. Expliclity set state to PLAYING and keep watching
    // state
    bin.set_state(gst::State::Playing)
        .expect("Error set bin state to playing");

    true
}

fn reconnect_bin(bin: &gst::Bin, ctx: &Arc<Mutex<Context>>) {
    {
        let mut ctx_lock = ctx.lock().unwrap();
        ctx_lock.reconnecting = false;
        ctx_lock.last_reconnect_time = Instant::now();
    }

    if bin.set_state(gst::State::Null).is_err() {
        log::error!("Cant set source bin {} state to NULL", bin.name());
        return;
    }

    if bin.sync_state_with_parent().is_err() {
        log::error!("Cant sync state with parent of source {}", bin.name());
    }

    let (ret, _state, _pending) = bin.state(gst::ClockTime::ZERO);
    if let Ok(succces) = ret {
        if succces == gst::StateChangeSuccess::Async
            || succces == gst::StateChangeSuccess::NoPreroll
        {
            let bin_week = bin.downgrade();
            let ctx_clone = ctx.clone();
            let timeout_id = glib::timeout_add(std::time::Duration::from_millis(20), move || {
                let bin = bin_week.upgrade().unwrap();
                let ret = watch_source_async_state_change(&bin, &ctx_clone);

                glib::ControlFlow::from(ret)
            });
            {
                let mut ctx_lock = ctx.lock().unwrap();
                ctx_lock.async_state_watch_timeout = Some(timeout_id);
            }
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            reconnecting: false,
            last_reconnect_time: Instant::now(),
            last_buffer_update: Instant::now(),
            async_state_watch_timeout: None,
            source_watch_timeout: None,
        }
    }
}

pub struct BinConectionManager {
    ctx: Arc<Mutex<Context>>,
}

impl BinConectionManager {
    pub fn new(bin: &gst::Bin) -> Self {
        let manager = Self {
            ctx: Arc::new(Mutex::new(Context::new())),
        };

        manager.setup_source_watch_timeout(bin);

        manager
    }

    pub fn setup_src_pad(&self, src_pad: &gst::Pad) {
        let ctx_week = Arc::downgrade(&self.ctx);
        src_pad.add_probe(gst::PadProbeType::BUFFER, move |_, info| {
            match &info.data {
                Some(gst::PadProbeData::Buffer(_)) => {
                    let ctx = ctx_week.upgrade().unwrap();
                    let mut ctx_lock = ctx.lock().unwrap();
                    ctx_lock.last_buffer_update = std::time::Instant::now();
                    ctx_lock.reconnecting = false;
                }
                _ => {}
            };

            gst::PadProbeReturn::Ok
        });
    }

    fn setup_source_watch_timeout(&self, bin: &gst::Bin) {
        let ctx_week = Arc::downgrade(&self.ctx);
        let bin_week = bin.downgrade();
        let source_watch_timeout = Some(glib::timeout_add(
            std::time::Duration::from_secs(1),
            move || {
                let bin = bin_week.upgrade().unwrap();
                let ctx = ctx_week.upgrade().unwrap();

                let reset_requierd = {
                    let ctx_lock = ctx.lock().unwrap();
                    let update_elapsed = ctx_lock.last_buffer_update.elapsed();
                    let reconnect_elapsed = ctx_lock.last_reconnect_time.elapsed();

                    if ctx_lock.reconnecting {
                        if reconnect_elapsed >= std::time::Duration::from_secs(30) {
                            log::warn!("Reconect failed from source {}, trying again", bin.name());
                            true
                        } else {
                            false
                        }
                    } else if update_elapsed >= std::time::Duration::from_secs(10)
                        && reconnect_elapsed >= std::time::Duration::from_secs(10)
                    {
                        log::warn!("No data from source {}, trying reconect", bin.name());
                        true
                    } else {
                        false
                    }
                };

                if reset_requierd {
                    reconnect_bin(&bin, &ctx);
                }

                glib::ControlFlow::Continue
            },
        ));

        self.ctx.lock().unwrap().source_watch_timeout = source_watch_timeout;
    }

    pub fn is_reconnecting(&self) -> bool {
        self.ctx.lock().unwrap().reconnecting
    }
}

impl Drop for BinConectionManager {
    fn drop(&mut self) {
        log::debug!("Drop BinConectionManager");
    }
}
