#![cfg_attr(feature = "windows_subsystem", windows_subsystem = "windows")]
#![allow(clippy::type_complexity)] // TODO

mod capturer;
mod fonts;
mod rw_condvar;
mod screen_dimension;
mod stready_redraw;
mod timers;

use std::{
    cell::Cell,
    sync::{atomic::AtomicU64, Arc},
    time::{Duration, Instant},
};

use capturer::Capturer;
use eframe::{egui, epi};
use egui::{
    style::Spacing, Color32, FontData, FontDefinitions, FontFamily, Rgba, RichText, TextStyle,
    TextureId, Ui, Vec2,
};
use fonts::RawFont;
use image::{Bgra, ImageBuffer, Pixel, RgbaImage};
use image_match::{
    buff::BuffMatcher,
    jinhillah::{JinHillahHpMatcher, JinHillahReapMatcher},
    Matcher,
};
use log::trace;
use screen_dimension::ScreenDimension;
use timers::{
    jinhillah::JinhillahTimer,
    match_agent::MatchAgent,
    vskill::{VSkillKind, VSkillTimer},
    Timer,
};

struct MatchOptions {
    jinhillah: bool,
    jinhillah_hard: bool,
    vskill: bool,
    vskill_kind: VSkillKind,
}

impl Default for MatchOptions {
    fn default() -> Self {
        Self {
            jinhillah: false,
            jinhillah_hard: true,
            vskill: false,
            vskill_kind: VSkillKind::FatalStrike,
        }
    }
}

#[derive(Default)]
struct MyEguiApp {
    capturer: Option<Result<Capturer, ()>>,
    last_get: Option<Instant>,
    dimension: ScreenDimension,
    preview_check: bool,
    preview_texture: Option<Cell<(TextureId, Vec2)>>,
    match_options: MatchOptions,
    timers: Vec<Box<dyn Timer>>,
    init_time: Option<Instant>,
    hidpi: bool,
    last_redraw: Option<Arc<AtomicU64>>,
    debug: bool,
}

impl epi::App for MyEguiApp {
    fn name(&self) -> &str {
        concat!("메이플스토리 타이머 ver. ", env!("CARGO_PKG_VERSION"))
    }

    fn setup(
        &mut self,
        ctx: &egui::CtxRef,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        let mut font_def = FontDefinitions::default();
        for font in assets_embedded::assets()
            .load_dir::<RawFont>("SpoqaHanSansNeo_OTF_original", false)
            .unwrap()
            .iter()
        {
            let font = font.unwrap();

            font_def.font_data.insert(
                String::from(font.id().rsplit_once('.').unwrap().1),
                FontData::from_owned(font.cloned().0),
            );
        }
        font_def.fonts_for_family.insert(
            FontFamily::Proportional,
            IntoIterator::into_iter([String::from("SpoqaHanSansNeo-Regular")])
                .chain(
                    font_def
                        .fonts_for_family
                        .get(&egui::FontFamily::Proportional)
                        .cloned()
                        .iter()
                        .flatten()
                        .cloned(),
                )
                .collect(),
        );
        // HACK: Use monospace font family as bold
        font_def.fonts_for_family.insert(
            FontFamily::Monospace,
            IntoIterator::into_iter([String::from("SpoqaHanSansNeo-Bold")])
                .chain(
                    font_def
                        .fonts_for_family
                        .get(&egui::FontFamily::Monospace)
                        .cloned()
                        .iter()
                        .flatten()
                        .cloned(),
                )
                .collect(),
        );

        font_def
            .family_and_size
            .insert(egui::TextStyle::Heading, (FontFamily::Proportional, 42.0));
        font_def
            .family_and_size
            .insert(egui::TextStyle::Body, (FontFamily::Proportional, 22.0));
        font_def
            .family_and_size
            .insert(egui::TextStyle::Small, (FontFamily::Proportional, 16.0));
        font_def
            .family_and_size
            .insert(egui::TextStyle::Button, (FontFamily::Proportional, 20.0));
        font_def
            .family_and_size
            .insert(egui::TextStyle::Monospace, (FontFamily::Monospace, 22.0));

        ctx.set_fonts(font_def);

        let mut style = (&*ctx.style()).clone();
        style.spacing = Spacing {
            item_spacing: Vec2::new(12.0, 12.0),
            ..Default::default()
        };
        ctx.set_style(style);

        self.init_time = Some(Instant::now());
        let redraw = Arc::new(AtomicU64::new(0));
        self.last_redraw = Some(Arc::clone(&redraw));
        stready_redraw::SteadyRedraw(frame.clone(), redraw, self.init_time.unwrap()).redraw_task();
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        self.last_redraw.as_ref().unwrap().store(
            Instant::now()
                .saturating_duration_since(self.init_time.unwrap())
                .as_millis() as u64,
            std::sync::atomic::Ordering::SeqCst,
        );
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if self.timers.is_empty() {
                        self.settings_ui(ctx, frame, ui);
                    } else {
                        self.timer_ui(ctx, frame, ui);
                    }
                });
        });
    }
}

impl MyEguiApp {
    fn timer_ui(&mut self, _ctx: &egui::CtxRef, _frame: &epi::Frame, ui: &mut Ui) {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
            if ui.button("정지하기").clicked() {
                self.timers.truncate(0);
            }
        });

        ui.style_mut().override_text_style = Some(TextStyle::Heading);

        for timer in &mut self.timers {
            ui.horizontal(|ui| {
                ui.label(format!("{}:", timer.text()));
                let remaining = timer.remaining_time();
                if let Some(remaining) = remaining {
                    let fmt = if remaining >= Duration::from_secs(60) {
                        format!(
                            "{:02}:{:02}",
                            remaining.as_secs() / 60,
                            remaining.as_secs() % 60
                        )
                    } else if remaining > Duration::from_secs(5) {
                        remaining.as_secs().to_string()
                    } else {
                        format!("{:.2}", remaining.as_secs_f64())
                    };

                    if remaining < timer.red_threshold() {
                        timer.wake();
                        ui.label(RichText::new(fmt).color(Color32::from_rgb(240, 30, 30)));
                    } else if remaining < timer.yellow_threshold() {
                        ui.label(RichText::new(fmt).color(Color32::from_rgb(240, 240, 30)));
                    } else {
                        ui.label(RichText::new(fmt).color(Color32::from_rgb(200, 200, 200)));
                    }
                } else {
                    ui.label(RichText::new("―").color(Color32::from_gray(60)));
                };
                if self.debug {
                    ui.style_mut().wrap = Some(true);
                    ui.label(RichText::new(timer.debug_string()).small());
                }
            });
        }
    }

    fn settings_ui(&mut self, _ctx: &egui::CtxRef, frame: &epi::Frame, ui: &mut Ui) {
        ui.heading("메이플스토리 타이머");
        let capturer = match self.capturer.as_ref() {
            Some(Ok(capturer)) => capturer,
            Some(Err(..)) => {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(180, 20, 0),
                        "메이플스토리 창을 찾을 수 없습니다.",
                    );
                    if ui.button("다시 시도하기").clicked() {
                        self.capturer = Some(Capturer::new(self.hidpi).map_err(|_| ()));
                    }
                });
                return;
            }
            None => {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("메이플스토리 창 찾기").clicked() {
                        self.capturer = Some(Capturer::new(self.hidpi).map_err(|_| ()));
                    }
                    ui.checkbox(&mut self.hidpi, "고해상도 모니터 모드")
                        .on_hover_text(RichText::new(
                        "메이플스토리 화면은 잘 잡히지만 화면 크기가 맞지 않을 때 시도해보세요.",
                    ).small());
                });
                return;
            }
        };

        let dims = (capturer.dims().0, capturer.dims().1);
        if dims == (0, 0) {
            ui.label("메이플스토리 창을 찾는 중입니다...");
            return;
        }

        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "메이플스토리 창을 찾았습니다. 크기: {}x{}",
                dims.0, dims.1
            ));
            if dims != (self.dimension.width(), self.dimension.height()) {
                if (dims.0 * 2 / 3, dims.1 * 2 / 3)
                    == (self.dimension.width(), self.dimension.height())
                {
                    warn_icon(
                        ui,
                        "창의 크기가 설정값과 다릅니다.\n\
                팁: 프로그램을 재시작한 후 고해상도 모니터 모드를 활성화해보세요.",
                    );
                } else {
                    warn_icon(ui, "창의 크기가 설정값과 다릅니다.");
                }
            }

            let elapsed = self
                .last_get
                .map(|x| Instant::now().saturating_duration_since(x) > Duration::from_millis(200))
                .unwrap_or(true);
            if self.preview_check && elapsed {
                trace!("Acquiring capturer");
                let img = capturer.lock_ref().read().clone();
                trace!("Released capturer");
                self.last_get = Some(Instant::now());
                if let Some(img) = img {
                    if img.pixels().count() > 0 {
                        let new_img = RgbaImage::from_fn(img.width(), img.height(), |x, y| {
                            img.get_pixel(x, y).to_rgba()
                        });
                        let size =
                            Vec2::new(new_img.width() as f32 / 4.0, new_img.height() as f32 / 4.0);
                        let texture = frame.alloc_texture(epi::Image::from_rgba_unmultiplied(
                            [new_img.width() as usize, new_img.height() as usize],
                            &new_img.into_raw(),
                        ));

                        if let Some(t) = &self.preview_texture {
                            let old_texture = t.replace((texture, size)).0;
                            frame.free_texture(old_texture);
                        } else {
                            self.preview_texture = Some(Cell::new((texture, size)));
                        }
                    }
                }
            }

            ui.checkbox(&mut self.preview_check, "화면 미리보기");

            if capturer.is_panicked() {
                error_icon(
                    ui,
                    "메이플스토리 화면 캡쳐 모듈이 비정상적으로 종료되었습니다. \
프로그램을 재시작해주세요.",
                );
            }
        });

        if self.preview_check {
            if let Some(texture) = &self.preview_texture {
                ui.image(texture.get().0, texture.get().1);
            }
        }

        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 48.0),
            egui::Layout::left_to_right().with_cross_align(egui::Align::Center),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.heading("옵션");
                    // TODO: Refactor this into method
                    let something = self.match_options.jinhillah || self.match_options.vskill;
                    ui.add_enabled_ui(something, |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(14.0);
                            if ui.button("시작하기").clicked() {
                                self.init_timers();
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.add_space(14.0);
                        ui.checkbox(&mut self.debug, "디버그 모드");
                    });
                });
            },
        );

        ui.add_enabled_ui(self.timers.is_empty(), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("메이플스토리 화면 크기:");
                ui.selectable_value(
                    &mut self.dimension,
                    ScreenDimension::X1280Y720,
                    ScreenDimension::X1280Y720.to_str(),
                );
                ui.selectable_value(
                    &mut self.dimension,
                    ScreenDimension::X1366Y768,
                    ScreenDimension::X1366Y768.to_str(),
                );
            });
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.match_options.jinhillah, "진힐라 타이머 사용하기");
                ui.selectable_value(&mut self.match_options.jinhillah_hard, false, "노말");
                ui.selectable_value(&mut self.match_options.jinhillah_hard, true, "하드");
            });
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(
                    &mut self.match_options.vskill,
                    "5차 스킬코어 타이머 사용하기",
                );
                ui.selectable_value(
                    &mut self.match_options.vskill_kind,
                    VSkillKind::FatalStrike,
                    "일격필살",
                );
            });
        });
    }

    fn init_timers(&mut self) {
        if self.match_options.jinhillah {
            self.timers.push(Box::new(JinhillahTimer::new(
                Arc::clone(self.capturer.as_mut().unwrap().as_mut().unwrap().cond()),
                Arc::clone(self.capturer.as_mut().unwrap().as_mut().unwrap().lock_ref()),
                self.capturer.as_mut().unwrap().as_mut().unwrap().dims(),
                !self.match_options.jinhillah_hard,
            )));
        }

        if self.match_options.vskill {
            self.timers.push(Box::new(VSkillTimer::new(
                Arc::clone(self.capturer.as_mut().unwrap().as_mut().unwrap().cond()),
                Arc::clone(self.capturer.as_mut().unwrap().as_mut().unwrap().lock_ref()),
                self.match_options.vskill_kind,
                self.capturer.as_mut().unwrap().as_mut().unwrap().dims(),
            )))
        }
    }
}

fn warn_icon(ui: &mut Ui, hover_message: impl Into<String>) {
    ui.colored_label(
        Rgba::from_rgb(0.5, 0.5, 0.1),
        RichText::new("[!]").text_style(TextStyle::Monospace),
    )
    .on_hover_text(RichText::new(hover_message).color(Rgba::from_gray(0.4)));
}

fn error_icon(ui: &mut Ui, hover_message: impl Into<String>) {
    if ui
        .colored_label(
            Rgba::from_rgb(0.8, 0.1, 0.1),
            RichText::new("[!]").text_style(TextStyle::Monospace),
        )
        .hovered()
    {
        ui.label(RichText::new(hover_message).color(Rgba::from_gray(0.4)));
    }
}

fn main() {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    env_logger::init();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |e| {
        let stderr = std::io::stderr();
        let _lock = stderr.lock();
        hook(e);
    }));

    <JinHillahHpMatcher as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::init();
    <JinHillahReapMatcher as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::init();
    <BuffMatcher as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::init();

    let app = MyEguiApp::default();
    assets_embedded::assets();
    let native_options = eframe::NativeOptions {
        always_on_top: true,
        decorated: true,
        initial_window_size: Some(Vec2::new(600.0, 420.0)),
        ..Default::default()
    };

    eframe::run_native(Box::new(app), native_options);
}
