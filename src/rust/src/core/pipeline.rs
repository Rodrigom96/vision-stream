use glib::IsA;
use gst::prelude::*;

fn setup_gst_pipeline_bus(pipeline: &gst::Pipeline, name: &str) {
    let name = name.to_string();

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    bus.set_sync_handler(move |_, msg| {
        match msg.view() {
            gst::MessageView::Error(err) => {
                log::error!(
                    "Error on pipeline of \"{}\" from {:?}: {} ({:?})",
                    name,
                    err.src().map_or("none".into(), |s| s.name()),
                    err.error(),
                    err.debug()
                );
            }
            _ => (),
        }

        gst::BusSyncReply::Pass
    });
}

pub struct Pipeline {
    pub pipeline: gst::Pipeline,
}

impl Pipeline {
    pub fn new(name: &str) -> Self {
        let pipeline = gst::Pipeline::new();

        setup_gst_pipeline_bus(&pipeline, name);

        Self { pipeline }
    }

    pub fn add(&self, element: &impl IsA<gst::Element>) -> Result<(), glib::error::BoolError> {
        self.pipeline.add(element)
    }

    pub fn add_many(
        &self,
        elements: impl IntoIterator<Item = impl AsRef<gst::Element>>,
    ) -> Result<(), glib::error::BoolError> {
        self.pipeline.add_many(elements)
    }

    pub fn set_state(
        &self,
        state: gst::State,
    ) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
        self.pipeline.set_state(state)
    }

    pub fn state(
        &self,
        timeout: impl Into<Option<gst::ClockTime>>,
    ) -> (
        Result<gst::StateChangeSuccess, gst::StateChangeError>,
        gst::State,
        gst::State,
    ) {
        self.pipeline.state(timeout)
    }
}
