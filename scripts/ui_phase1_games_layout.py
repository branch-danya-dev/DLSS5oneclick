from pathlib import Path

path = Path("src/gui.rs")
s = path.read_text(encoding="utf-8")

start_marker = "        // ── header ────────────────────────────────────────────────\n"
end_marker = "        // ── grid, grouped by store ────────────────────────────────\n"

start = s.index(start_marker)
end = s.index(end_marker, start)

new_block = r'''        // ── page heading ───────────────────────────────────────────
        let installed_n = self.meta.values().filter(|m| m.installed).count();
        let total_n = self.games.len();
        let dx12_n = self
            .meta
            .values()
            .filter(|m| m.api.starts_with("DirectX 12"))
            .count();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new("Games").font(t::sora(20.0)).color(t::TEXT));
                ui.label(
                    RichText::new("Install and manage DLSS 5 for detected games.")
                        .font(t::plex(12.0))
                        .color(t::TEXT_MUTED),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let summary = if self.scanning {
                    "Scanning Steam, Epic, GOG and Xbox…".to_owned()
                } else {
                    format!(
                        "{total_n} games · {installed_n} installed · {dx12_n} DX12"
                    )
                };
                ui.label(
                    RichText::new(summary)
                        .font(t::plex_medium(11.5))
                        .color(t::TEXT_DIM),
                );
            });
        });
        ui.add_space(12.0);

        // ── desktop toolbar ────────────────────────────────────────
        let toolbar_w = ui.available_width();
        let btn = |text: &str, accent: bool| {
            egui::Button::new(
                RichText::new(text)
                    .font(t::plex_medium(12.0))
                    .color(if accent { t::BG } else { t::TEXT_OFF }),
            )
            .fill(if accent {
                t::ACCENT
            } else {
                Color32::TRANSPARENT
            })
            .stroke(Stroke::new(
                1.0,
                if accent { t::ACCENT } else { t::BORDER_STRONG },
            ))
            .corner_radius(CornerRadius::same(8))
            .min_size(Vec2::new(96.0, 36.0))
        };

        if toolbar_w >= 760.0 {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let actions_w = 104.0 + 112.0 + 96.0 + 24.0;
                let search_w = (ui.available_width() - actions_w).max(180.0);
                let search = egui::TextEdit::singleline(&mut self.search)
                    .font(t::plex(12.0))
                    .hint_text(RichText::new("Search games…").color(t::TEXT_DIM))
                    .desired_width(search_w);
                ui.add_sized([search_w, 36.0], search);

                if ui.add(btn("Add a game", false)).clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Executables", &["exe", "bin"])
                        .pick_file()
                    {
                        self.add_game(p, ui.ctx());
                    }
                }
                if ui.add(btn("Add a folder", false)).clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .set_title("Pick the game's install folder")
                        .pick_folder()
                    {
                        self.add_game(p, ui.ctx());
                    }
                }
                if ui
                    .add_enabled(!self.scanning && !self.running, btn("Rescan", true))
                    .clicked()
                {
                    self.start_scan(ui.ctx());
                }
            });
        } else {
            let search = egui::TextEdit::singleline(&mut self.search)
                .font(t::plex(12.0))
                .hint_text(RichText::new("Search games…").color(t::TEXT_DIM))
                .desired_width(ui.available_width());
            ui.add_sized([ui.available_width(), 36.0], search);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if ui.add(btn("Add a game", false)).clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Executables", &["exe", "bin"])
                        .pick_file()
                    {
                        self.add_game(p, ui.ctx());
                    }
                }
                if ui.add(btn("Add a folder", false)).clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .set_title("Pick the game's install folder")
                        .pick_folder()
                    {
                        self.add_game(p, ui.ctx());
                    }
                }
                if ui
                    .add_enabled(!self.scanning && !self.running, btn("Rescan", true))
                    .clicked()
                {
                    self.start_scan(ui.ctx());
                }
            });
        }
        ui.add_space(18.0);

'''

s = s[:start] + new_block + s[end:]
path.write_text(s, encoding="utf-8")
