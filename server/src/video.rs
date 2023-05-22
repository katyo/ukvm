use crate::Result;
use linux_video::{types::*, Device, Stream};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{spawn, sync::watch, time};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct VideoConfig {
    /// Device (video0)
    pub device: String,

    /// Default width
    #[serde(default = "default_width")]
    pub width: u32,

    /// Default height
    #[serde(default = "default_height")]
    pub height: u32,
}

fn default_width() -> u32 {
    1920
}

fn default_height() -> u32 {
    1080
}

pub type VideoFrame = Arc<Vec<u8>>;

pub type VideoSource = watch::Receiver<VideoFrame>;

pub struct Video {
    frame_receiver: VideoSource,
}

impl Video {
    /// Attach listener
    pub fn frames(&self) -> VideoSource {
        self.frame_receiver.clone()
    }

    /// Create video input from config
    pub async fn new(config: &VideoConfig) -> Result<Self> {
        let (frame_sender, frame_receiver) = watch::channel(Arc::new(Vec::default()));

        let device = Self::prepare(config).await?;

        spawn(async move {
            log::info!("Initialize capturing video");

            let mut stream: Option<Stream<In, Mmap>> = None;

            while !frame_sender.is_closed() {
                if stream.is_some() {
                    // video stream started
                    match stream.as_ref().unwrap().next().await {
                        Ok(buffer) => {
                            let buffer = buffer.lock();
                            let data: &[u8] = buffer.as_ref();
                            if data.len() > 4 {
                                let _ = frame_sender.send(Arc::new(data.to_vec()));
                            }
                        }
                        Err(error) => {
                            // Stop streaming on error
                            log::error!("Error when capturing video: {error}");
                            stream = None;
                        }
                    }
                    if frame_sender.receiver_count() < 2 {
                        // Stop streaming when no sinks
                        log::info!("Stop capturing video");
                        stream = None;
                    }
                } else {
                    // video stream stopped
                    if frame_sender.receiver_count() > 1 {
                        // receiver attached, start streaming
                        match device.stream::<In, Mmap>(ContentType::Video, 5) {
                            Ok(stm) => {
                                log::info!("Start capturing video");
                                stream = Some(stm);
                            }
                            Err(error) => log::error!("Unable to capture video due to: {error}"),
                        }
                    } else {
                        time::sleep(time::Duration::from_millis(500)).await
                    }
                }
            }
            log::info!("Finalize capturing video");
        });

        Ok(Self { frame_receiver })
    }

    async fn has_mjpeg(device: &Device) -> Result<bool> {
        let mut formats = device.formats(BufferType::VideoCapture);

        while let Some(format) = formats.fetch_next().await? {
            if format.type_() == BufferType::VideoCapture
                && format.flags().contains(FmtFlag::Compressed)
                && format.pixel_format() == FourCc::Mjpeg
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn prepare(config: &VideoConfig) -> Result<Device> {
        let device = Device::open(&config.device).await?;

        let caps = device.capabilities().await?;

        if !caps.capabilities().contains(CapabilityFlag::VideoCapture) {
            Err("Device doesn't capable for capturing video")?;
        }

        if !Self::has_mjpeg(&device).await? {
            Err("Device hasn't support MJPEG for capturing video")?;
        }

        let mut format = device.format(BufferType::VideoCapture).await?;
        let pixfmt = format.try_mut::<PixFormat>().unwrap();
        let bits = 16;

        pixfmt
            .set_width(config.width)
            .set_height(config.height)
            .set_bytes_per_line(config.width * bits / 8)
            .set_size_image(config.width * config.height * bits / 8)
            .set_pixel_format(FourCc::Mjpeg)
            .set_color_space(ColorSpace::Jpeg);

        device.set_format(&mut format).await?;

        Ok(device)
    }
}
