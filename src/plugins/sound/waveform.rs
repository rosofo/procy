use std::time::Duration;

use super::raytrace::{CastData, Raycaster, Raycasts};
use crate::prelude::*;
use __std_iter::empty;
use bevy::math::cubic_splines::LinearSpline;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Host, Stream, SupportedBufferSize,
};
use tracing::instrument;
use triple_buffer::{triple_buffer, Input};

#[derive(Reflect)]
pub struct Waveform {
    spline: LinearSpline<Vec2>,
}

impl Waveform {
    pub fn samples(&self) -> Option<Vec<f32>> {
        Some(
            // self.spline
            //     .to_curve()
            //     .ok()?
            //     .iter_positions(511)
            //     .map(|pos| (pos.y * 5.0).sin())
            //     .zip(
            self.spline
                .points
                .iter()
                .map(|point| (0..).map(|i| (i as f32 * point.y).sin()))
                .fold(vec![0.0; 512], |a, b| {
                    a.iter().zip(b).map(|(x, y)| x + y).collect_vec()
                })
                .iter()
                // )
                // .map(|(x, y)| (x + y) / self.spline.points.len() as f32)
                .copied()
                .collect(),
        )
    }
    pub fn from_rays<'a>(rays: impl IntoIterator<Item = &'a CastData>) -> Self {
        let mut t = 0.0;
        let mut points = rays
            .into_iter()
            .map(|ray| {
                let seg = ray.point - ray.origin;
                let amplitude = seg.y;
                let out = Vec2::new(t, amplitude);
                t += seg.length();
                out
            })
            .collect_vec();

        let max_t = points
            .iter()
            .map(|p| p.x)
            .max_by(|t1, t2| t1.partial_cmp(t2).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        let (min_a, max_a) = points.iter().map(|p| p.y).minmax().into_option().unwrap();
        for point in points.iter_mut() {
            point.x = point.x.remap(0.0, max_t, 0.0, 1.0);
            point.y = point.y.remap(min_a, max_a, -1.0, 1.0);
        }

        let spline = LinearSpline::new(points);

        Self { spline }
    }
}

pub(super) fn trace_waves(
    mut input: ResMut<InputBuffer>,
    casts: Query<&Raycasts>,
    mut timer: Local<Timer>,
    time: Res<Time>,
    mut last: Local<Vec<f32>>,
) {
    if last.len() != 512 {
        *last = vec![0.0; 512];
    }
    timer.tick(time.delta());
    if timer.finished() {
        input.input_buffer().fill(0.0);
        let mut total = 0;
        for casts in casts.iter() {
            for rays in casts.0.iter() {
                let waveform = Waveform::from_rays(rays);
                if let Some(samples) = waveform.samples() {
                    total += 1;
                    for (i, (in_samp, samp)) in input
                        .input_buffer()
                        .iter_mut()
                        .zip(samples.iter())
                        .enumerate()
                    {
                        if let Some(last_samp) = last.get(i) {
                            let mixed = (*samp + *last_samp) / 2.0;
                            *in_samp = mixed;
                            last[i] = *samp;
                        }
                        if samp.is_nan() {
                            continue;
                        }
                        *in_samp += samp;
                    }
                }
            }
        }
        if total > 1 {
            for samp in input.input_buffer().iter_mut() {
                *samp /= total as f32;
                if samp.is_nan() {
                    *samp = 0.0;
                }
            }
        }
        debug!("publish samples for {} raytraces", total);
        if let MinMaxResult::MinMax(min, max) = input.input_buffer().iter().minmax() {
            debug!("stats: min {}, max {}", min, max);
        }
        if input.publish() {
            warn!("overwrote buffer that was never read by audio thread");
        };

        timer.set_duration(Duration::from_millis(10));
        timer.reset();
    }
}

pub struct CpalState {
    host: Host,
    device: Device,
    stream: Stream,
}

#[derive(Resource, Deref, DerefMut)]
pub struct InputBuffer(Input<Vec<f32>>);

impl CpalState {
    pub(super) fn setup() -> (Self, InputBuffer) {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .next()
            .expect("no supported config?!")
            .with_max_sample_rate();
        let buffer_size = match supported_config.buffer_size() {
            SupportedBufferSize::Range { min, max } => *max,
            _ => 512,
        };
        let (input, mut output) = triple_buffer(&vec![0.0; buffer_size as usize]);
        let stream = device
            .build_output_stream(
                &supported_config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let buf = output.read();
                    for (i, sample) in data.iter_mut().enumerate() {
                        *sample = buf[i];
                    }
                },
                move |err| {
                    // react to errors here.
                    panic!("stream error: {:?}", err);
                },
                None, // None=blocking, Some(Duration)=timeout
            )
            .unwrap();
        stream.play().unwrap();
        (
            CpalState {
                host,
                device,
                stream,
            },
            InputBuffer(input),
        )
    }
}
