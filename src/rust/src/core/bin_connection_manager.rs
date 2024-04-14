use std::sync::{Arc, Mutex};
use std::time::Instant;

use gst::prelude::*;

#[derive(Debug, PartialEq)]
pub enum RetryReason {
    Timeout,
    StateChangeFailure,
}

struct Context {
    pub reconnecting: bool,
    pub pending_restart: bool,
    pub last_buffer_update: std::time::Instant,
    pub last_reconnect_time: std::time::Instant,
    pub async_state_watch_timeout: Option<glib::SourceId>,
    pub source_watch_timeout: Option<glib::SourceId>,
    // For timing out the source and shutting it down to restart it
    pub restart_timeout: Option<gst::SingleShotClockId>,
    // For restarting the source after shutting it down
    pub pending_restart_timeout: Option<gst::SingleShotClockId>,
}

impl Drop for Context {
    fn drop(&mut self) {
        if let Some(source_id) = self.source_watch_timeout.take() {
            source_id.remove();
        };
        if let Some(source_id) = self.async_state_watch_timeout.take() {
            source_id.remove();
        }
        if let Some(timeout) = self.restart_timeout.take() {
            timeout.unschedule();
        }
        if let Some(timeout) = self.pending_restart_timeout.take() {
            timeout.unschedule();
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
        log::debug!("Bin {} state change failed", bin.name());
        let mut ctx_guard = ctx.lock().unwrap();
        ctx_guard.async_state_watch_timeout = None;
        drop(ctx_guard);
        handle_source_error(bin, ctx, RetryReason::StateChangeFailure);
        return false;
    }

    // Bin successfully changed state to PLAYING. Stop watching state
    if state == gst::State::Playing {
        let mut ctx_guard = ctx.lock().unwrap();
        ctx_guard.reconnecting = false;
        ctx_guard.async_state_watch_timeout = None;
        drop(ctx_guard);
        return false;
    }

    // Bin has stopped ASYNC state change but has not gone into
    // PLAYING. Expliclity set state to PLAYING and keep watching
    // state
    bin.set_state(gst::State::Playing)
        .expect("Error set bin state to playing");

    true
}

fn handle_source_error(source: &gst::Bin, ctx: &Arc<Mutex<Context>>, reason: RetryReason) {
    log::debug!("Handling source {} error: {:?}", source.name(), reason);

    if ctx.lock().expect("no context").pending_restart {
        log::debug!("Source is already pending restart");
        return;
    }

    // Unschedule pending timeout
    let mut ctx_binding = ctx.lock();
    let ctx_guard = ctx_binding.as_mut().expect("no context");
    if let Some(timeout) = ctx_guard.restart_timeout.take() {
        timeout.unschedule();
    }

    ctx_guard.pending_restart = true;
    ctx_guard.reconnecting = true;
    drop(ctx_binding);

    let ctx_week = Arc::downgrade(ctx);
    source.call_async(move |element| {
        element.set_state(gst::State::Null);
        let ctx = ctx_week.upgrade().expect("no context");

        // Sleep for 5s before retrying
        let clock = gst::SystemClock::obtain();
        let wait_time = clock.time().unwrap() + gst::ClockTime::from_seconds(5);
        let mut ctx_binding = ctx.lock();
        let ctx_guard = ctx_binding.as_mut().expect("no context");
        assert!(ctx_guard.pending_restart_timeout.is_none());
        drop(ctx_binding);

        let timeout = clock.new_single_shot_id(wait_time);
        let element_weak = element.downgrade();
        let ctx_week = Arc::downgrade(&ctx);
        timeout
            .wait_async(move |_clock, _time, _id| {
                let Some(element) = element_weak.upgrade() else {
                    return;
                };
                let ctx = ctx_week.upgrade().expect("no context");

                let mut ctx_guard = ctx.lock().expect("no context");
                ctx_guard.pending_restart = false;
                ctx_guard.last_reconnect_time = Instant::now();
                ctx_guard.pending_restart_timeout = None;
                if let Some(timeout) = ctx_guard.restart_timeout.take() {
                    timeout.unschedule();
                }
                drop(ctx_guard);

                if element.sync_state_with_parent().is_err() {
                    log::error!("Source failed to change state");
                    element.set_state(gst::State::Null);
                    handle_source_error(&element, &ctx, RetryReason::StateChangeFailure);
                } else {
                    let (ret, _state, _pending) = element.state(gst::ClockTime::ZERO);
                    let mut ctx_guard = ctx.lock().expect("no context");
                    if let Ok(succces) = ret {
                        if succces == gst::StateChangeSuccess::Async
                            || succces == gst::StateChangeSuccess::NoPreroll
                        {
                            let bin_week = element.downgrade();
                            let ctx_week = Arc::downgrade(&ctx);
                            let timeout_id = glib::timeout_add(
                                std::time::Duration::from_millis(20),
                                move || {
                                    let Some(ctx) = ctx_week.upgrade() else {
                                        return glib::ControlFlow::Break;
                                    };

                                    let bin = bin_week.upgrade().unwrap();
                                    let ret = watch_source_async_state_change(&bin, &ctx);

                                    glib::ControlFlow::from(ret)
                                },
                            );
                            {
                                ctx_guard.async_state_watch_timeout = Some(timeout_id);
                            }
                        } else {
                            ctx_guard.reconnecting = false;
                        }
                    }
                    drop(ctx_guard);
                }
            })
            .expect("Failed to wait async");

        let mut ctx_guard = ctx.lock().expect("no context");
        ctx_guard.pending_restart_timeout = Some(timeout);
        drop(ctx_guard);
    })
}

impl Context {
    pub fn new() -> Self {
        Self {
            reconnecting: false,
            pending_restart: false,
            last_reconnect_time: Instant::now(),
            last_buffer_update: Instant::now(),
            async_state_watch_timeout: None,
            source_watch_timeout: None,
            restart_timeout: None,
            pending_restart_timeout: None,
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

                let retry_reason = {
                    let ctx_lock = ctx.lock().unwrap();
                    let update_elapsed = ctx_lock.last_buffer_update.elapsed();
                    let reconnect_elapsed = ctx_lock.last_reconnect_time.elapsed();

                    if ctx_lock.reconnecting {
                        None
                    } else if update_elapsed >= std::time::Duration::from_secs(10)
                        && reconnect_elapsed >= std::time::Duration::from_secs(10)
                    {
                        log::debug!(
                            "update_elapsed: {:?}, reconnect_elapsed: {:?}",
                            update_elapsed,
                            reconnect_elapsed
                        );
                        Some(RetryReason::Timeout)
                    } else {
                        None
                    }
                };

                if let Some(reason) = retry_reason {
                    handle_source_error(&bin, &ctx, reason)
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
