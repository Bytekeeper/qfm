#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{
    egui,
    egui::text::LayoutJob,
    egui::{Color32, FontFamily, FontId, Stroke, TextEdit, TextFormat},
};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

fn main() {
    let options = eframe::NativeOptions {
        decorated: false,
        resizable: false,
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

struct MyApp {
    history: Vec<PathBuf>,
    history_pos: usize,
    filter: String,
    selected: i32,
    dir: PathBuf,
}

impl Default for MyApp {
    fn default() -> Self {
        let dir = PathBuf::from(".").canonicalize().unwrap();

        Self {
            history: vec![dir.clone()],
            history_pos: 0,
            filter: "".to_string(),
            selected: 0,
            dir,
        }
    }
}

struct Entry {
    file_name: String,
    path: PathBuf,
    metadata: fs::Metadata,
}

impl MyApp {
    fn push_dir(&mut self, path: PathBuf) {
        self.dir = path;
        self.filter = "".to_string();
        self.history.truncate(self.history_pos + 1);
        self.history_pos = self.history.len();
        self.history.push(self.dir.clone());
    }
}

enum Part {
    NonMatch(String),
    Match(String),
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        {
            let input = ctx.input();
            if input.key_down(egui::Key::Escape) {
                frame.quit();
            }
            if input.key_pressed(egui::Key::ArrowUp) {
                self.selected -= 1;
            }
            if input.key_pressed(egui::Key::ArrowDown) {
                self.selected += 1;
            }
            if input.key_pressed(egui::Key::Home) {
                self.selected = 0;
            }
            if input.modifiers.alt && input.key_pressed(egui::Key::ArrowLeft) {
                self.history_pos = self.history_pos.saturating_sub(1);
                self.dir = self.history[self.history_pos].clone();
            }
            if input.modifiers.alt && input.key_pressed(egui::Key::ArrowRight) {
                self.history_pos = (self.history_pos + 1).min(self.history.len() - 1);
                self.dir = self.history[self.history_pos].clone();
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            // let mut s = ui.style().deref().clone();
            // s.override_font_id.as_mut().unwrap().size = 16.0;
            // ui.set_style(s);
            ui.add(TextEdit::singleline(&mut self.filter).lock_focus(true))
                .request_focus();
            ui.label(self.dir.to_string_lossy().to_string());
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut entries: Vec<_> = fs::read_dir(&self.dir)
                    .unwrap()
                    .flatten()
                    .flat_map(|file| {
                        file.metadata().ok().map(|metadata| Entry {
                            file_name: file.file_name().to_string_lossy().to_string(),
                            path: file.path(),
                            metadata,
                        })
                    })
                    .collect();
                entries.sort_by_key(|e| {
                    std::cmp::Reverse(
                        e.metadata
                            .accessed()
                            .or_else(|_| e.metadata.modified())
                            .unwrap(),
                    )
                });
                let mut idx = 0;
                let mut selected = ui.input().key_pressed(egui::Key::Enter);
                if let Some(parent) = self
                    .dir
                    .parent()
                    .map(|it| it.canonicalize())
                    .transpose()
                    .ok()
                    .flatten()
                {
                    selected |= ui
                        .selectable_value(&mut self.selected, idx as i32, "..")
                        .clicked();
                    if self.selected == idx && selected {
                        self.push_dir(parent);
                    }
                }
                idx += 1;
                for entry in entries {
                    let mut entry_iter = entry.file_name.chars();
                    let mut show = true;
                    let mut hits = vec![];
                    'outer: for c in self.filter.chars() {
                        while let Some(d) = entry_iter.next() {
                            let d_str = d.to_string();
                            if c.to_string().to_uppercase().cmp(&d_str.to_uppercase())
                                == Ordering::Equal
                            {
                                hits.push(Part::Match(d_str));
                                continue 'outer;
                            } else {
                                hits.push(Part::NonMatch(d_str));
                            }
                        }
                        show = false;
                    }
                    if show {
                        while let Some(d) = entry_iter.next() {
                            hits.push(Part::NonMatch(d.to_string()));
                        }
                        let mut job = LayoutJob::default();
                        let color = if entry.metadata.is_dir() {
                            Color32::BROWN
                        } else {
                            Color32::GRAY
                        };
                        let default_format = TextFormat {
                            color,
                            font_id: FontId::new(16.0, FontFamily::Monospace),
                            ..Default::default()
                        };
                        for h in hits {
                            match h {
                                Part::Match(c) => {
                                    job.append(
                                        &c,
                                        0.0,
                                        TextFormat {
                                            color: Color32::BLACK,
                                            underline: Stroke {
                                                width: 1.0,
                                                color: Color32::BLACK,
                                            },
                                            font_id: FontId::new(16.0, FontFamily::Monospace),
                                            ..Default::default()
                                        },
                                    );
                                }
                                Part::NonMatch(c) => {
                                    job.append(&c, 0.0, default_format.clone());
                                }
                            }
                        }
                        let response = ui.selectable_value(&mut self.selected, idx as i32, job);

                        selected |= response.clicked();
                        if idx == self.selected {
                            response.scroll_to_me(None);
                            if selected {
                                if !ui.input().modifiers.alt
                                    && entry.metadata.is_dir()
                                    && !response.double_clicked()
                                {
                                    self.push_dir(entry.path);
                                } else {
                                    std::thread::spawn(|| open::that(entry.path).ok());
                                }
                            }
                        }
                        idx += 1;
                    }
                }
                self.selected = self.selected.max(0).min(idx - 1);
            });
            // ui.horizontal(|ui| {
            //     ui.label("Your name: ");
            //     ui.text_edit_singleline(&mut self.name);
            // });
            // ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            // if ui.button("Click each year").clicked() {
            //     self.age += 1;
            // }
            // ui.label(format!("Hello '{}', age {}", self.name, self.age));
        });
    }
}
