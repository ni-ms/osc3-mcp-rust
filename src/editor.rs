use nih_plug::prelude::{Editor, util};
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{ViziaState, ViziaTheming, create_vizia_editor};

use crate::{SineParams, Waveform};

pub const NOTO_SANS: &str = "Noto Sans";

#[derive(Lens)]
struct Data {
    params: Arc<SineParams>,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (650, 550))
}

pub(crate) fn create(
    params: Arc<SineParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _setter| {
        Data {
            params: params.clone(),
        }
        .build(cx);

        VStack::new(cx, |cx| {
            // Title
            Label::new(cx, "Triple Oscillator Synth")
                .font_family(vec![FamilyOwned::Named(String::from(NOTO_SANS))])
                .font_weight(FontWeightKeyword::Bold)
                .font_size(22.0)
                .height(Pixels(40.0))
                .color(Color::rgb(240, 240, 240))
                .alignment(Alignment::Center);

            // Oscillator 1 Section
            VStack::new(cx, |cx| {
                Label::new(cx, "Oscillator 1")
                    .font_weight(FontWeightKeyword::Bold)
                    .font_size(14.0)
                    .color(Color::rgb(150, 150, 255))
                    .height(Pixels(20.0));

                VStack::new(cx, |cx| {
                    // Frequency control
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Frequency")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |params| &params.frequency1)
                            .width(Pixels(300.0))
                            .height(Pixels(20.0));
                    })
                    .space(Pixels(10.0));

                    // Gain control
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Gain")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |params| &params.gain1)
                            .width(Pixels(200.0))
                            .height(Pixels(20.0));
                    })
                    .space(Pixels(10.0));

                    // Waveform selection using ParamButton
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Waveform")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);

                        // Note: Vizia doesn't have a direct equivalent to egui's ComboBox
                        // We'll use a simple display of the current waveform for now
                        Label::new(
                            cx,
                            Data::params.map(|p| match p.waveform1.value() {
                                Waveform::Sine => "Sine".to_string(),
                                Waveform::Square => "Square".to_string(),
                                Waveform::Triangle => "Triangle".to_string(),
                                Waveform::Sawtooth => "Sawtooth".to_string(),
                            }),
                        )
                        .width(Pixels(80.0))
                        .height(Pixels(20.0))
                        .background_color(Color::rgb(60, 80, 120))
                        .corner_radius(Pixels(3.0))
                        .alignment(Alignment::Center)
                        .font_size(11.0);
                    })
                    .space(Pixels(10.0));
                })
                .space(Pixels(5.0));
            })
            .space(Pixels(5.0))
            .padding(Pixels(10.0))
            .background_color(Color::rgb(45, 45, 55))
            .corner_radius(Pixels(4.0));

            // Oscillator 2 Section
            VStack::new(cx, |cx| {
                Label::new(cx, "Oscillator 2")
                    .font_weight(FontWeightKeyword::Bold)
                    .font_size(14.0)
                    .color(Color::rgb(150, 255, 150))
                    .height(Pixels(20.0));

                VStack::new(cx, |cx| {
                    // Frequency control
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Frequency")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |params| &params.frequency2)
                            .width(Pixels(300.0))
                            .height(Pixels(20.0));
                    })
                    .space(Pixels(10.0));

                    // Gain control
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Gain")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |params| &params.gain2)
                            .width(Pixels(200.0))
                            .height(Pixels(20.0));
                    })
                    .space(Pixels(10.0));

                    // Waveform display
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Waveform")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);

                        Label::new(
                            cx,
                            Data::params.map(|p| match p.waveform2.value() {
                                Waveform::Sine => "Sine".to_string(),
                                Waveform::Square => "Square".to_string(),
                                Waveform::Triangle => "Triangle".to_string(),
                                Waveform::Sawtooth => "Sawtooth".to_string(),
                            }),
                        )
                        .width(Pixels(80.0))
                        .height(Pixels(20.0))
                        .background_color(Color::rgb(60, 120, 80))
                        .corner_radius(Pixels(3.0))
                        .alignment(Alignment::Center)
                        .font_size(11.0);
                    })
                    .space(Pixels(10.0));
                })
                .space(Pixels(5.0));
            })
            .space(Pixels(5.0))
            .padding(Pixels(10.0))
            .background_color(Color::rgb(45, 55, 45))
            .corner_radius(Pixels(4.0));

            // Oscillator 3 Section
            VStack::new(cx, |cx| {
                Label::new(cx, "Oscillator 3")
                    .font_weight(FontWeightKeyword::Bold)
                    .font_size(14.0)
                    .color(Color::rgb(255, 150, 150))
                    .height(Pixels(20.0));

                VStack::new(cx, |cx| {
                    // Frequency control
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Frequency")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |params| &params.frequency3)
                            .width(Pixels(300.0))
                            .height(Pixels(20.0));
                    })
                    .space(Pixels(10.0));

                    // Gain control
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Gain")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |params| &params.gain3)
                            .width(Pixels(200.0))
                            .height(Pixels(20.0));
                    })
                    .space(Pixels(10.0));

                    // Waveform display
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Waveform")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);

                        Label::new(
                            cx,
                            Data::params.map(|p| match p.waveform3.value() {
                                Waveform::Sine => "Sine".to_string(),
                                Waveform::Square => "Square".to_string(),
                                Waveform::Triangle => "Triangle".to_string(),
                                Waveform::Sawtooth => "Sawtooth".to_string(),
                            }),
                        )
                        .width(Pixels(80.0))
                        .height(Pixels(20.0))
                        .background_color(Color::rgb(120, 60, 80))
                        .corner_radius(Pixels(3.0))
                        .alignment(Alignment::Center)
                        .font_size(11.0);
                    })
                    .space(Pixels(10.0));
                })
                .space(Pixels(5.0));
            })
            .space(Pixels(5.0))
            .padding(Pixels(10.0))
            .background_color(Color::rgb(55, 45, 45))
            .corner_radius(Pixels(4.0));

            // Instructions
            Label::new(cx, "Use host automation or MIDI CC to change waveforms")
                .font_size(10.0)
                .color(Color::rgb(150, 150, 150))
                .height(Pixels(15.0))
                .alignment(Alignment::Center);
        })
        .space(Pixels(12.0))
        .padding(Pixels(15.0))
        .background_color(Color::rgb(30, 30, 35));
    })
}
