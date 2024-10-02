use eframe::{egui, App, Frame, CreationContext};
use crate::{caster, receiver};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, RwLock};
use eframe::egui::{Rect, Pos2, Color32, UiBuilder, Image, Widget};
use tokio::runtime::Runtime;
use image::{ImageBuffer, Rgba};
use scrap::{Capturer, Display};
use std::time::Duration;
use std::thread;

#[derive(Debug, Clone)]
enum Modality {
    Caster,
    Receiver,
}

pub struct MyApp {
    mode: Option<Modality>,
    caster_address: String,
    status_message: String,
    caster_running: Arc<AtomicBool>,
    receiver_running: Arc<AtomicBool>,
    stop_signal: Arc<AtomicBool>,
    start_pos: Option<Pos2>,
    selecting_area: bool,
    selected_area: Option<Rect>,
    screenshot: Option<egui::TextureHandle>,
    error_message: Arc<RwLock<Option<String>>>,
    is_error: Arc<AtomicBool>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            mode: None,
            caster_address: String::from(""),
            status_message: String::from("Seleziona una modalità per iniziare."),
            caster_running: Arc::new(AtomicBool::new(false)),
            receiver_running:  Arc::new(AtomicBool::new(false)),
            stop_signal: Arc::new(AtomicBool::new(false)),
            start_pos: None,
            selecting_area: false,
            selected_area: None,
            screenshot: None,
            error_message: Arc::new(RwLock::new(None)),
            is_error: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl MyApp {
    pub fn new(_cc: &CreationContext<'_>) -> Self {
        Default::default()
    }

    fn display_error(&self, ui: &mut egui::Ui) {
        if self.is_error.load(Ordering::SeqCst) {
            if let Some(error) = self.error_message.read().unwrap().as_ref() {
                ui.label(egui::RichText::new(error).color(egui::Color32::RED));
            }
        }
    }
    fn clear_error(&self) {
        *self.error_message.write().unwrap() = None;
        self.is_error.store(false, Ordering::SeqCst);
    }

    fn set_error(&self, error: String) {
        *self.error_message.write().unwrap() = Some(error);
        self.is_error.store(true, Ordering::SeqCst);
    }
    fn handle_selection(&mut self, ctx: &egui::Context) {
        if self.selecting_area {
            ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::Crosshair);
            let response = ctx.input(|i| {
                let pos = i.pointer.hover_pos();
                let pressed = i.pointer.primary_pressed();
                let released = i.pointer.primary_released();
                (pos, pressed, released)
            });

            if let (Some(current_pos), pressed, released) = response {
                if pressed && self.start_pos.is_none() {
                    self.start_pos = Some(current_pos);
                } else if released && self.start_pos.is_some() {
                    let start = self.start_pos.unwrap();
                    self.selected_area = Some(Rect::from_two_pos(start, current_pos));
                    self.selecting_area = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                    self.status_message = format!("Area selezionata: {:?}", self.selected_area.unwrap());
                }
                if self.selecting_area && (pressed || released) {
                    ctx.request_repaint();
                }
            }
        }else {
            ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::Default);
        }
    }


    fn capture_screenshot(&mut self, ctx: &egui::Context) {
        let display = match Display::primary() {
            Ok(display) => display,
            Err(e) => {
                return;
            }
        };

        let mut capturer = match scrap::Capturer::new(display) {
            Ok(capturer) => capturer,
            Err(e) => {
                eprintln!("Errore nella creazione del capturer: {}", e);
                return;
            }
        };

        let width = capturer.width();
        let height = capturer.height();

        let frame = loop {
            match capturer.frame() {
                Ok(frame) => break frame,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    // Attendere un po' prima di riprovare
                    thread::sleep(Duration::from_millis(100)); // Attende 100ms
                    continue; // Riprova a catturare il frame
                }
                Err(e) => {
                    eprintln!("Errore nella cattura del frame: {}", e);
                    return;
                }
            }
        };

        let mut img_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width as u32, height as u32);
        for (i, pixel) in img_buffer.pixels_mut().enumerate() {
            let idx = i * 4;
            *pixel = Rgba([frame[idx + 2], frame[idx + 1], frame[idx], 255]);
        }

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [width, height],
            &img_buffer.into_raw(),
        );

        self.screenshot = Some(ctx.load_texture(
            "screenshot",
            color_image,
            egui::TextureOptions::LINEAR,
        ));
    }

}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {


        if self.selecting_area {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(Color32::from_rgba_unmultiplied(0, 0, 0, 200)))
                .show(ctx, |ui| {
                    if let Some(texture) = &self.screenshot {
                        let size=ui.available_size();
                        let image=Image::from_texture(texture).fit_to_exact_size(size).tint(Color32::from_rgba_unmultiplied(110, 110, 110, 200));
                        image.ui(ui);
                    }
                    let screen_rect = ui.max_rect();
                    let center_x = screen_rect.center().x;
                    let center_y = screen_rect.center().y;
                    let rect = Rect::from_center_size(
                        Pos2::new(center_x, center_y),
                        egui::vec2(200.0, 50.0)
                    );
                    ui.allocate_new_ui(UiBuilder::max_rect(Default::default(), rect), |ui| {
                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                            ui.colored_label(egui::Color32::WHITE, egui::RichText::new("Clicca e trascina per selezionare l'area").strong());
                        });
                    },
                    );
                    self.handle_selection(ctx);
                    if let Some(start) = self.start_pos {
                        if let Some(current) = ui.input(|i| i.pointer.hover_pos()) {
                            let rect = Rect::from_two_pos(start, current);
                            ui.painter().rect_stroke(rect, 0.0, (2.0, egui::Color32::WHITE));
                            ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 50));
                        }
                    }
                });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.display_error(ui);
                ui.heading("Screencast Application");
                ui.horizontal(|ui| {
                    if ui.button("Caster").clicked() {
                        self.clear_error();
                        self.mode = Some(Modality::Caster);
                        self.caster_running.store(false,Ordering::SeqCst);
                        self.receiver_running.store(false,Ordering::SeqCst);
                        self.stop_signal.store(false, Ordering::SeqCst);
                        self.selecting_area = false;
                        self.selected_area = None;
                        self.status_message = "Modalità selezionata: Caster".to_string();
                    }
                    if ui.button("Receiver").clicked() {
                        self.clear_error();
                        self.mode = Some(Modality::Receiver);
                        self.caster_running.store(false,Ordering::SeqCst);
                        self.receiver_running.store(false,Ordering::SeqCst);
                        self.stop_signal.store(false, Ordering::SeqCst);
                        self.selecting_area = false;
                        self.selected_area = None;
                        self.status_message = "Modalità selezionata: Receiver".to_string();
                    }
                });

                if let Some(ref mode) = self.mode {
                    match mode {
                        Modality::Caster => {
                            ui.horizontal(|ui| {
                                ui.label("Indirizzo caster: es.127.0.0.1:12345 in locale o tra più dispositivi 192.168.165.219:8080");
                                ui.text_edit_singleline(&mut self.caster_address);
                            });

                            if !self.caster_running.load(Ordering::SeqCst) {
                                self.status_message="Modalità selezionata: Caster".to_string();
                                if ui.button("Avvia").clicked() {
                                    self.clear_error(); // Pulisce l'errore quando si preme Avvia
                                    // ... codice esistente ...

                                    self.status_message = "Avviando il caster...".to_string();
                                    self.caster_running.store(true,Ordering::SeqCst);
                                    self.stop_signal.store(false, Ordering::SeqCst);

                                    let stop_signal = self.stop_signal.clone();
                                    let ctx = ctx.clone();
                                    let selected_area = self.selected_area;  // Pass the selected area
                                    let caster_address = self.caster_address.clone();  // Use the IP input
                                    self.selected_area = None;
                                    let error_message = self.error_message.clone();
                                    let is_error = self.is_error.clone();
                                    let is_running=self.caster_running.clone();
                                    std::thread::spawn(move || {
                                        Runtime::new().unwrap().block_on(async {
                                            if let Err(e) = caster::start_caster(&caster_address, stop_signal, selected_area).await {
                                                let error = format!("Errore nel caster: {}", e);
                                                *error_message.write().unwrap() = Some(error);
                                                is_error.store(true, Ordering::SeqCst);
                                                is_running.store(false,Ordering::SeqCst);
                                                eprintln!("Errore: {}", e);
                                            }else {
                                                is_running.store(false, Ordering::SeqCst);


                                            }
                                        });
                                        ctx.request_repaint();
                                    });
                                }

                                if ui.button("Seleziona area").clicked() {
                                    self.capture_screenshot(ctx);
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                                    self.selecting_area = true;
                                    self.start_pos = None;
                                    self.status_message = "Clicca e trascina per selezionare l'area".to_string();
                                }
                            } else {
                                if ui.button("Stop").clicked() {
                                    self.status_message = "Interrompendo il caster...".to_string();
                                    self.stop_signal.store(true, Ordering::SeqCst);
                                    self.caster_running.store(false,Ordering::SeqCst);

                                    self.status_message = "Caster interrotto.".to_string();
                                }


                                ui.label(egui::RichText::new("\nShortcuts:\nFn + F1 --> Metti in pausa lo stream;\nFn + F2 --> Blank screen;\nESC --> Interrompi lo stream\n")
                                    .color(egui::Color32::BLACK));
                            }
                        }

                        Modality::Receiver => {
                            ui.horizontal(|ui| {
                                ui.label("Indirizzo caster: es.127.0.0.1:12345 in locale o tra più dispositivi 192.168.165.219:8080");
                                ui.text_edit_singleline(&mut self.caster_address);
                            });

                            if !self.receiver_running.load(Ordering::SeqCst) {
                                self.status_message="Modalità selezionata: Receiver".to_string();
                                if ui.button("Avvia").clicked()   {
                                    self.clear_error();
                                    let addr = self.caster_address.clone();
                                    self.status_message = "Connettendo al caster...".to_string();
                                    self.receiver_running.store(true,Ordering::SeqCst) ;
                                    self.stop_signal.store(false, Ordering::SeqCst);

                                    let stop_signal = self.stop_signal.clone();
                                    let ctx = ctx.clone();
                                    let error_message = self.error_message.clone();
                                    let is_error = self.is_error.clone();
                                    let is_running=self.receiver_running.clone();
                                    std::thread::spawn(move || {
                                        Runtime::new().unwrap().block_on(async {
                                            if let Err(e) = receiver::receive_frame(&addr, stop_signal).await {
                                                let error = format!("Errore nel caster: {}", e);
                                                *error_message.write().unwrap() = Some(error);
                                                is_error.store(true, Ordering::SeqCst);
                                                is_running.store(false,Ordering::SeqCst);
                                                eprintln!("Errore: {}", e);
                                            }else {
                                                is_running.store(false,Ordering::SeqCst);
                                            }
                                        });
                                        ctx.request_repaint();
                                    });
                                }
                            } else {
                                if ui.button("Stop").clicked() {
                                    self.status_message = "Interrompendo il receiver...".to_string();
                                    self.stop_signal.store(true, Ordering::SeqCst);
                                    self.receiver_running.store(false,Ordering::SeqCst);
                                }
                            }
                        }
                    }
                }

                ui.label(&self.status_message);
            });
        }
    }
}