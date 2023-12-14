use crate::image::Image;

pub fn pull_appsink_image(appsink: &gst_app::AppSink) -> Option<Image> {
    let sample = appsink.pull_sample().unwrap();

    let caps = sample.caps().unwrap();
    let cap_stucture = caps.structure(0).unwrap();
    let width = cap_stucture.get::<i32>("width").unwrap();
    let height = cap_stucture.get::<i32>("height").unwrap();

    let buffer = sample.buffer().unwrap();
    let map = buffer.map_readable().unwrap();
    let data = map.as_slice().to_vec();

    Some(Image {
        width,
        height,
        channels: 3,
        data,
    })
}
