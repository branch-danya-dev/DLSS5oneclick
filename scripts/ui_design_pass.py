from pathlib import Path

path = Path("src/gui.rs")
s = path.read_text(encoding="utf-8")

old_constants = """const CARD_GAP: f32 = 12.0;
const LIST_COVER_W: f32 = 92.0;
const LIST_CARD_H: f32 = 132.0;
const LIST_CARD_MIN_W: f32 = 420.0;"""
new_constants = """const CARD_GAP: f32 = 14.0;
const LIST_COVER_W: f32 = 118.0;
const LIST_CARD_H: f32 = 176.0;
const LIST_CARD_MIN_W: f32 = 560.0;"""
if old_constants not in s:
    raise SystemExit("game-card constants block not found")
s = s.replace(old_constants, new_constants, 1)

start = s.index("    fn games_page(&mut self, ui: &mut egui::Ui) {")
end = s.index("    /// Horizontal list card", start)
new_games = r'''    fn games_page(&mut self, ui: &mut egui::Ui) {
        if self.store_icons.is_empty() {
            self.store_icons = load_store_icons(ui.ctx());
        }
        let installed_n = self.meta.values().filter(|m| m.installed).count();
        let total_n = self.games.len();
        let available_n = total_n.saturating_sub(installed_n);

        // Broad hero: title/subtitle on the left, three equal status blocks on the right.
        let hero_w = ui.available_width();
        Frame::new()
            .fill(t::SURFACE_ALT)
            .stroke(Stroke::new(1.0, t::BORDER_STRONG))
            .corner_radius(t::card_rounding())
            .inner_margin(Margin::symmetric(22, 18))
            .show(ui, |ui| {
                ui.set_width((hero_w - 44.0).max(320.0));
                let inner_w = ui.available_width();
                if inner_w >= 900.0 {
                    let gap = 24.0;
                    let left_w = (inner_w * 0.50).max(360.0);
                    let right_w = (inner_w - left_w - gap).max(420.0);
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(left_w, 78.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.add_space(3.0);
                                ui.label(
                                    RichText::new(self.tr(T::YourGames))
                                        .font(t::sora(25.0))
                                        .color(t::TEXT),
                                );
                                ui.add_space(5.0);
                                ui.label(
                                    RichText::new(self.tr(T::YourGamesSub))
                                        .font(t::plex(13.5))
                                        .color(t::TEXT_SECONDARY),
                                );
                            },
                        );
                        ui.add_space(gap);
                        ui.allocate_ui_with_layout(
                            Vec2::new(right_w, 74.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.columns(3, |cols| {
                                    ui_c::summary_stat_dot(
                                        &mut cols[0],
                                        self.tr(T::TotalGames),
                                        total_n.to_string(),
                                        t::PRIMARY,
                                    );
                                    ui_c::summary_stat_dot(
                                        &mut cols[1],
                                        self.tr(T::InstalledCount),
                                        installed_n.to_string(),
                                        t::SUCCESS,
                                    );
                                    ui_c::summary_stat_dot(
                                        &mut cols[2],
                                        self.tr(T::AvailableCount),
                                        available_n.to_string(),
                                        t::WARNING,
                                    );
                                });
                            },
                        );
                    });
                } else {
                    ui.label(
                        RichText::new(self.tr(T::YourGames))
                            .font(t::sora(24.0))
                            .color(t::TEXT),
                    );
                    ui.label(
                        RichText::new(self.tr(T::YourGamesSub))
                            .font(t::plex(13.0))
                            .color(t::TEXT_SECONDARY),
                    );
                    ui.add_space(14.0);
                    ui.columns(3, |cols| {
                        ui_c::summary_stat_dot(
                            &mut cols[0],
                            self.tr(T::TotalGames),
                            total_n.to_string(),
                            t::PRIMARY,
                        );
                        ui_c::summary_stat_dot(
                            &mut cols[1],
                            self.tr(T::InstalledCount),
                            installed_n.to_string(),
                            t::SUCCESS,
                        );
                        ui_c::summary_stat_dot(
                            &mut cols[2],
                            self.tr(T::AvailableCount),
                            available_n.to_string(),
                            t::WARNING,
                        );
                    });
                }
            });
        ui.add_space(16.0);

        // Toolbar: the search field consumes the flexible space, actions stay compact.
        let search_hint = self.tr(T::SearchGames);
        let add_game_l = self.tr(T::AddGame);
        let add_folder_l = self.tr(T::AddFolder);
        let rescan_l = self.tr(T::Rescan);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let actions_w = 154.0 + 146.0 + 138.0 + 30.0;
            let search_w = (ui.available_width() - actions_w).max(260.0);
            let search = egui::TextEdit::singleline(&mut self.search)
                .font(t::plex(13.0))
                .hint_text(RichText::new(search_hint).color(t::TEXT_DIM))
                .desired_width(search_w);
            ui.add_sized([search_w, 40.0], search);
            if ui
                .add(ui_c::secondary_button(add_game_l).min_size(Vec2::new(154.0, 40.0)))
                .clicked()
            {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Executables", &["exe", "bin"])
                    .pick_file()
                {
                    self.add_game(p, ui.ctx());
                }
            }
            if ui
                .add(ui_c::secondary_button(add_folder_l).min_size(Vec2::new(146.0, 40.0)))
                .clicked()
            {
                if let Some(p) = rfd::FileDialog::new()
                    .set_title(self.tr(T::PickGameFolder))
                    .pick_folder()
                {
                    self.add_game(p, ui.ctx());
                }
            }
            if ui
                .add_enabled(
                    !self.scanning && !self.running,
                    ui_c::primary_button(rescan_l).min_size(Vec2::new(138.0, 40.0)),
                )
                .clicked()
            {
                self.start_scan(ui.ctx());
            }
        });
        ui.add_space(18.0);

        let needle = self.search.trim().to_ascii_lowercase();
        let mut clicked: Option<(PathBuf, usize)> = None;
        let mut forgotten: Option<PathBuf> = None;
        let mut update = false;
        let section_ours = self.tr(T::InstalledByTool);
        let section_manual = self.tr(T::AddedByYou);
        let empty_title = self.tr(T::NoGamesFound);
        let empty_hint = self.tr(T::NoGamesHint);

        egui::ScrollArea::vertical()
            .max_height(ui.available_height())
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                let sections: [Option<Store>; 6] = [
                    None,
                    Some(Store::Manual),
                    Some(Store::Steam),
                    Some(Store::Epic),
                    Some(Store::Gog),
                    Some(Store::Xbox),
                ];
                for section in sections {
                    let idx: Vec<usize> = self
                        .games
                        .iter()
                        .enumerate()
                        .filter(|(i, g)| {
                            let ours = self.meta.get(i).is_some_and(|m| m.installed);
                            let in_section = match section {
                                None => ours,
                                Some(st) => g.store == st && !ours,
                            };
                            in_section
                                && (needle.is_empty()
                                    || g.title.to_ascii_lowercase().contains(&needle))
                        })
                        .map(|(i, _)| i)
                        .collect();
                    if idx.is_empty() {
                        continue;
                    }

                    let section_w = ui.available_width();
                    Frame::new()
                        .fill(t::SURFACE)
                        .stroke(Stroke::new(1.0, t::BORDER))
                        .corner_radius(t::card_rounding())
                        .inner_margin(Margin::same(16))
                        .show(ui, |ui| {
                            ui.set_width((section_w - 32.0).max(320.0));
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 9.0;
                                ui.label(
                                    RichText::new(match section {
                                        None => section_ours,
                                        Some(Store::Manual) => section_manual,
                                        Some(st) => st.label(),
                                    })
                                    .font(t::plex_semibold(15.0))
                                    .color(t::TEXT),
                                );
                                ui_c::chip(
                                    ui,
                                    &idx.len().to_string(),
                                    ui_c::ChipTone::Neutral,
                                );
                            });
                            ui.add_space(12.0);

                            let inner_w = ui.available_width();
                            let cols = if inner_w >= 1160.0 { 2 } else { 1 };
                            let card_w = if cols == 2 {
                                (inner_w - CARD_GAP) / 2.0
                            } else {
                                inner_w
                            };

                            for row in idx.chunks(cols) {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = CARD_GAP;
                                    for &i in row {
                                        let action = self.game_card_list(ui, card_w, i);
                                        if action == CardAction::Forget {
                                            forgotten = Some(self.games[i].dir.clone());
                                        }
                                        if matches!(action, CardAction::Open | CardAction::Update) {
                                            update = action == CardAction::Update;
                                            let g = &self.games[i];
                                            let path = match game::resolve_target(&g.dir) {
                                                Ok(_) => g.dir.clone(),
                                                Err(_) => match &g.exe_hint {
                                                    Some(e)
                                                        if e.is_file()
                                                            && game::exe_bitness(e).is_ok() =>
                                                    {
                                                        e.clone()
                                                    }
                                                    _ => g.dir.clone(),
                                                },
                                            };
                                            clicked = Some((path, i));
                                        }
                                    }
                                });
                                ui.add_space(8.0);
                            }
                        });
                }

                if !self.scanning && self.games.is_empty() {
                    ui.add_space(48.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(empty_title)
                                .font(t::plex(14.0))
                                .color(t::TEXT_MUTED),
                        );
                        ui.label(
                            RichText::new(empty_hint)
                                .font(t::plex(12.5))
                                .color(t::TEXT_DIM),
                        );
                    });
                }
            });

        if let Some(p) = forgotten {
            library::forget_added(&p);
            self.games
                .retain(|g| !(g.store == Store::Manual && g.dir == p));
        }
        if let Some((p, index)) = clicked {
            if update {
                self.update_game(p, index);
            } else {
                self.open_game(p);
            }
        }
    }
'''
s = s[:start] + new_games + "\n" + s[end:]

card_start = s.index("    /// Horizontal list card", start)
card_end = s.index("    fn settings_page", card_start)
card = s[card_start:card_end]
card = card.replace(".fill(t::SURFACE)", ".fill(t::SURFACE_ALT)", 1)
card = card.replace(".inner_margin(Margin::same(10))", ".inner_margin(Margin::same(12))", 1)
card = card.replace(
    "RichText::new(&g.title)\n                                .font(t::plex_semibold(14.0))",
    "RichText::new(&g.title)\n                                .font(t::plex_semibold(15.0))",
    1,
)
card = card.replace(
    "ui.add_space(4.0);\n                    ui.horizontal_wrapped",
    "ui.add_space(7.0);\n                    ui.horizontal_wrapped",
    1,
)
card = card.replace(
    "ui.add_space(6.0);\n                    ui.horizontal(|ui| {",
    "ui.add_space(10.0);\n                    ui.horizontal(|ui| {",
    1,
)
card = card.replace(
    "ui_c::primary_button(primary).min_size(Vec2::new(160.0, 34.0))",
    "ui_c::primary_button(primary).min_size(Vec2::new(182.0, 38.0))",
    1,
)
card = card.replace(
    "ui_c::secondary_button(open_l).min_size(Vec2::new(120.0, 34.0))",
    "ui_c::secondary_button(open_l).min_size(Vec2::new(132.0, 38.0))",
    1,
)
s = s[:card_start] + card + s[card_end:]

path.write_text(s, encoding="utf-8")
