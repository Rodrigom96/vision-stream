use gst::prelude::*;

use crate::errors::Error;

pub fn add_bin_ghost_pad_named(
    bin: &gst::Bin,
    elem: &gst::Element,
    pad_name: &str,
    ghost_pad_name: &str,
) -> Result<(), Error> {
    let pad = match elem.static_pad(pad_name) {
        Some(pad) => pad,
        None => {
            panic!("Could not find {} pad", pad_name);
        }
    };
    let ghost_pad = gst::GhostPad::builder_with_target(&pad)?
        .name(ghost_pad_name)
        .build();
    bin.add_pad(&ghost_pad)?;

    Ok(())
}

pub fn add_bin_ghost_pad(bin: &gst::Bin, elem: &gst::Element, pad_name: &str) -> Result<(), Error> {
    add_bin_ghost_pad_named(bin, elem, pad_name, pad_name)
}
