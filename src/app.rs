// use std::sync::mpsc;
use std::time::{Duration, Instant};

use egui::collapsing_header::CollapsingHeader;
use egui_demo_lib::easy_mark::easy_mark;
use scale_info::{form::PortableForm, Variant};
use serde::{Deserialize, Serialize};
use subxt::metadata::types::{PalletMetadata, StorageEntryMetadata};
use subxt::Metadata;
use subxt::{client::OnlineClient, config::SubstrateConfig};
use tokio::sync::mpsc::{self, error::TryRecvError, UnboundedReceiver, UnboundedSender};

static APP_KEY: &str = "by.alestsurko.substrate-api-explorer";

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ApiExplorer {
    url: String,
    #[serde(skip)]
    response: Option<Metadata>,
    #[serde(skip)]
    request_error: Option<String>,
    #[serde(skip)]
    timeout: Timeout,
    #[serde(skip)]
    sender: UnboundedSender<String>,
    #[serde(skip)]
    receiver: UnboundedReceiver<Result<Metadata, String>>,
}

impl Default for ApiExplorer {
    fn default() -> Self {
        let (sender, mut receiver): (UnboundedSender<String>, UnboundedReceiver<String>) =
            mpsc::unbounded_channel();
        let (sender2, receiver2): (
            UnboundedSender<Result<Metadata, String>>,
            UnboundedReceiver<Result<Metadata, String>>,
        ) = mpsc::unbounded_channel();

        tokio::task::spawn(async move {
            loop {
                let url = receiver.recv().await.unwrap();
                match OnlineClient::<SubstrateConfig>::from_url(url).await {
                    Ok(client) => {
                        sender2.send(Ok(client.metadata())).unwrap();
                    }
                    Err(e) => {
                        sender2.send(Err(err_to_string(e))).unwrap();
                    }
                }
            }
        });

        Self {
            response: None,
            request_error: None,
            timeout: Timeout::default(),
            url: String::new(),
            sender,
            receiver: receiver2,
        }
    }
}

impl ApiExplorer {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    pub fn native_options() -> eframe::NativeOptions {
        let window_size = (600.0, 600.0);
        eframe::NativeOptions {
            resizable: false,
            min_window_size: Some(window_size.into()),
            max_window_size: Some(window_size.into()),
            ..Default::default()
        }
    }

    fn check_timeout_disable(&mut self, ui: &mut egui::Ui) {
        if self.timeout.passed() {
            ui.set_enabled(true);
        } else {
            ui.set_enabled(false);
            ui.label(format!(
                "Waiting for response {} seconds",
                self.timeout.remaining().as_secs() + 1
            ));
        }
    }

    fn render_request(&mut self, ui: &mut egui::Ui) {
        let mut is_valid = url::Url::parse(&self.url).is_ok();

        ui.horizontal_centered(|ui| {
            let button = egui::Button::new("⟳");

            if ui.add_enabled(is_valid, button).clicked() {
                if let Err(e) = self.request_metadata() {
                    self.request_error = Some(e);
                }
            }

            if ui.text_edit_singleline(&mut self.url).changed() {
                is_valid = url::Url::parse(&self.url).is_ok();
            }
        });

        if let Some(err) = &self.request_error {
            self.timeout.reset();
            ui.set_enabled(true);
            ui.colored_label(egui::Color32::RED, err);
        }

        if !is_valid && !self.url.is_empty() {
            ui.colored_label(egui::Color32::RED, "Invalid URL");
        }
    }

    fn request_metadata(&mut self) -> Result<(), String> {
        self.request_error = None;
        self.response = None;
        self.timeout.start();

        self.sender.send(self.url.clone()).map_err(err_to_string)
    }

    fn render_response(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if let Err(e) = self.check_response(ctx) {
            self.request_error = Some(e);
        }

        if let Some(response) = &self.response {
            ui.set_enabled(true);
            egui::ScrollArea::vertical().show(ui, |ui| {
                response.pallets().for_each(|metadata| {
                    Self::render_pallet_metadata(&metadata, ui);
                });
            });
        }
    }

    fn check_response(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if self.response.is_some() {
            return Ok(());
        }

        ctx.request_repaint();
        self.response = match self.receiver.try_recv() {
            Ok(resp) => Some(resp.map_err(err_to_string)?),

            Err(TryRecvError::Empty) if !self.timeout.passed() => {
                return Ok(());
            }

            Err(TryRecvError::Disconnected) => {
                return Err("The sender disconnected! Please, restart the app".to_string())
            }

            _ => {
                return Ok(());
            }
        };

        self.timeout.reset();

        Ok(())
    }

    fn render_pallet_metadata(pallet: &PalletMetadata<'_>, ui: &mut egui::Ui) {
        CollapsingHeader::new(pallet.name()).show(ui, |ui| {
            ui.set_width(ui.available_width());

            if !pallet.docs().is_empty() {
                let docs = concat_docs(pallet.docs());
                easy_mark(ui, &docs);
            }

            if let Some(metadata) = pallet.storage() {
                CollapsingHeader::new("Storage").show(ui, |ui| {
                    for md in metadata.entries() {
                        Self::render_storage_metadata(md, ui);
                    }
                });
            }

            Self::render_variants(pallet.call_variants(), "Calls", ui);
            Self::render_variants(pallet.event_variants(), "Events", ui);
            Self::render_variants(pallet.error_variants(), "Errors", ui);
        });
    }

    fn render_storage_metadata(metadata: &StorageEntryMetadata, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(metadata.name())
                .code()
                .color(egui::Color32::GREEN)
                .size(16.0),
        );

        if !metadata.docs().is_empty() {
            ui.add_space(10.0);
            let docs = concat_docs(metadata.docs());
            easy_mark(ui, &docs);
        }

        ui.add_space(15.0);
    }

    fn render_variants(
        variants: Option<&'_ [Variant<PortableForm>]>,
        name: &str,
        ui: &mut egui::Ui,
    ) {
        if let Some(variants) = variants {
            CollapsingHeader::new(name).show(ui, |ui| {
                for variant in variants {
                    Self::render_variant(variant, ui);
                }
            });
        }
    }

    fn render_variant(variant: &Variant<PortableForm>, ui: &mut egui::Ui) {
        let fields = variant.fields.iter().fold(String::new(), |mut acc, field| {
            field.name.as_ref().map(|name| {
                field.type_name.as_ref().map(|ty_name| {
                    if !acc.is_empty() {
                        acc.push_str(", ");
                    }

                    acc.push_str(&format!("{}: {}", name, ty_name));
                })
            });

            acc
        });

        let text = if !fields.is_empty() {
            format!("{}({})", variant.name, fields)
        } else {
            variant.name.clone()
        };

        ui.label(
            egui::RichText::new(text)
                .code()
                .color(egui::Color32::GREEN)
                .size(16.0),
        );
        ui.add_space(10.0);

        if !variant.docs.is_empty() {
            ui.add_space(10.0);
            let docs = concat_docs(&variant.docs);
            easy_mark(ui, &docs);
        }

        ui.add_space(15.0);
    }
}

fn concat_docs(lines: &[String]) -> String {
    lines.iter().fold(String::new(), |mut acc, line| {
        acc.push_str(line);
        acc.push('\n');
        acc
    })
}

fn err_to_string<T: std::fmt::Display>(e: T) -> String {
    format!("{}", e)
}

impl eframe::App for ApiExplorer {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("")
            .exact_height(30.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                self.check_timeout_disable(ui);
                self.render_request(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_response(ctx, ui);
        });
    }
}

struct Timeout {
    duration: Duration,
    start: Instant,
}

impl Default for Timeout {
    fn default() -> Self {
        let duration = Duration::from_secs(5);
        Self {
            duration,
            start: Instant::now() - duration,
        }
    }
}

impl Timeout {
    fn start(&mut self) {
        self.start = Instant::now();
    }

    fn reset(&mut self) {
        self.start = Instant::now() - self.duration;
    }

    fn passed(&self) -> bool {
        self.start.elapsed() > self.duration
    }

    fn remaining(&self) -> Duration {
        self.duration - self.start.elapsed()
    }
}
