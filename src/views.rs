// src/views.rs
use crate::app::{ActivePage, AuthViewMode, PetMedsApp};
use crate::controllers::PetMedsController;
use crate::dao::PetMedsDao;
use crate::models::*;
use crate::pyrsutils::prelude::*;
use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};

const TEAL: Color32 = Color32::from_rgb(28, 151, 146);

pub struct PetMedsViews;

impl PetMedsViews {
    pub fn render(app: &mut PetMedsApp, ctx: &egui::Context) {
        if app.current_user.is_none() {
            Self::render_auth(app, ctx);
        } else {
            Self::render_header(app, ctx);
            Self::render_bottom_nav(app, ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    Self::render_main_content(app, ui);
                });
            });
            Self::render_modals(app, ctx);
        }
        Self::render_alert_modal(app, ctx);
    }

    fn badge(ui: &mut egui::Ui, text: &str, rgb: [u8; 3]) {
        let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        egui::Frame::none()
            .fill(color)
            .rounding(8.0)
            .inner_margin(egui::Margin::symmetric(8.0, 2.0))
            .show(ui, |ui| {
                ui.label(RichText::new(text).color(Color32::WHITE).small().strong());
            });
    }

    // ---------------- Tela de Autenticação (RF001 - RF003) ----------------

    fn render_auth(app: &mut PetMedsApp, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.heading(
                    RichText::new("🐾 PetMeds Desktop")
                        .size(28.0)
                        .color(TEAL)
                        .strong(),
                );
                ui.label("Controle Colaborativo de Medicação Pet (SQLite Local ACID)");
                ui.add_space(20.0);

                egui::Frame::group(ui.style())
                    .rounding(14.0)
                    .inner_margin(24.0)
                    .show(ui, |ui| {
                        ui.set_max_width(380.0);
                        match app.auth_mode {
                            AuthViewMode::Login | AuthViewMode::Register => {
                                ui.horizontal(|ui| {
                                    ui.selectable_value(
                                        &mut app.auth_mode,
                                        AuthViewMode::Login,
                                        "Entrar",
                                    );
                                    ui.selectable_value(
                                        &mut app.auth_mode,
                                        AuthViewMode::Register,
                                        "Cadastrar",
                                    );
                                });
                                ui.separator();

                                if app.auth_mode == AuthViewMode::Register {
                                    ui.label("Nome Completo:");
                                    ui.text_edit_singleline(&mut app.auth_name);
                                    ui.add_space(6.0);
                                }
                                ui.label("E-mail:");
                                ui.text_edit_singleline(&mut app.auth_email);
                                ui.add_space(6.0);
                                ui.label("Senha:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.auth_pass).password(true),
                                );

                                if app.auth_mode == AuthViewMode::Register {
                                    ui.add_space(6.0);
                                    ui.label("Confirmar Senha:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut app.auth_confirm)
                                            .password(true),
                                    );
                                }
                                ui.add_space(14.0);

                                let btn_txt = if app.auth_mode == AuthViewMode::Login {
                                    "Entrar"
                                } else {
                                    "Cadastrar e Entrar"
                                };
                                if ui
                                    .add_sized(
                                        [340.0, 34.0],
                                        egui::Button::new(RichText::new(btn_txt).strong())
                                            .fill(TEAL),
                                    )
                                    .clicked()
                                {
                                    if app.auth_mode == AuthViewMode::Login {
                                        match PetMedsDao::authenticate(
                                            &app.conn,
                                            &app.auth_email,
                                            &app.auth_pass,
                                        ) {
                                            Ok(u) => {
                                                app.current_user = Some(u);
                                                app.active_pet_id = "all".into();
                                                app.refresh_data();
                                            }
                                            Err(e) => app.alert("Falha na autenticação", e),
                                        }
                                    } else if app.auth_pass != app.auth_confirm {
                                        app.alert(
                                            "Erro no cadastro",
                                            "As senhas digitadas não coincidem.",
                                        );
                                    } else {
                                        match PetMedsDao::register_user(
                                            &app.conn,
                                            &app.auth_name,
                                            &app.auth_email,
                                            &app.auth_pass,
                                        ) {
                                            Ok(u) => {
                                                app.current_user = Some(u);
                                                app.active_pet_id = "all".into();
                                                app.refresh_data();
                                            }
                                            Err(e) => app.alert("Erro no cadastro", e),
                                        }
                                    }
                                }
                                if ui.link("Esqueci minha senha / Redefinir local").clicked() {
                                    app.auth_mode = AuthViewMode::Forgot;
                                }
                            }
                            AuthViewMode::Forgot => {
                                ui.heading("Redefinir Senha Local");
                                ui.label("Informe o e-mail cadastrado e a nova senha:");
                                ui.text_edit_singleline(&mut app.auth_email);
                                ui.label("Nova Senha:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.auth_pass).password(true),
                                );
                                ui.add_space(10.0);
                                if ui.button("Salvar Nova Senha").clicked() {
                                    match PetMedsDao::reset_password_local(
                                        &app.conn,
                                        &app.auth_email,
                                        &app.auth_pass,
                                    ) {
                                        Ok(_) => {
                                            app.alert(
                                                "Senha atualizada",
                                                "Sua senha foi redefinida com sucesso.",
                                            );
                                            app.auth_mode = AuthViewMode::Login;
                                        }
                                        Err(e) => app.alert("Erro", e),
                                    }
                                }
                                if ui.button("Voltar ao Login").clicked() {
                                    app.auth_mode = AuthViewMode::Login;
                                }
                            }
                        }
                    });
            });
        });
    }

    // ---------------- Cabeçalho e Navegação Inferior ----------------

    fn render_header(app: &mut PetMedsApp, ctx: &egui::Context) {
        let fam_info = app
            .current_family
            .as_ref()
            .map(|f| (f.id.clone(), f.name.clone()));
        let families = app.user_families.clone();
        let role_label = app.role_policy().nome_papel();
        let user_btn_label = app.current_user.as_ref().map(|u| {
            let initial = u.name.chars().next().unwrap_or('U');
            format!("👤 {} ({})", u.name, initial)
        });

        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("🐾 PetMeds").color(TEAL).strong());
                ui.separator();

                if let Some((mut selected_fid, fam_name)) = fam_info {
                    let mut changed = false;
                    egui::ComboBox::from_id_salt("header_fam_select")
                        .selected_text(format!("🏠 {fam_name}"))
                        .show_ui(ui, |ui| {
                            for (fid, fname, _) in &families {
                                if ui
                                    .selectable_value(&mut selected_fid, fid.clone(), fname)
                                    .clicked()
                                {
                                    changed = true;
                                }
                            }
                        });
                    if changed {
                        PetMedsDao::set_pref(&app.conn, "active_family_id", &selected_fid);
                        app.active_pet_id = "all".into();
                        app.refresh_data();
                    }
                }

                Self::badge(ui, role_label, [28, 151, 146]);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(lbl) = user_btn_label {
                        if ui.button(lbl).clicked() {
                            app.page = ActivePage::Usuario;
                        }
                    }
                });
            });
        });
    }

    fn render_bottom_nav(app: &mut PetMedsApp, ctx: &egui::Context) {
        let is_temp = app.role_policy().0 == "cuidador_temporario";
        egui::TopBottomPanel::bottom("bottom_nav").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                if !is_temp {
                    ui.selectable_value(&mut app.page, ActivePage::Pets, "🐾 Pets");
                    ui.separator();
                    ui.selectable_value(&mut app.page, ActivePage::Remedios, "💊 Medicamentos");
                    ui.separator();
                }
                ui.selectable_value(&mut app.page, ActivePage::Dashboard, "📊 Dashboard");
                if !is_temp {
                    ui.separator();
                    ui.selectable_value(&mut app.page, ActivePage::Tratamentos, "📋 Tratamentos");
                }
                ui.separator();
                ui.selectable_value(&mut app.page, ActivePage::Config, "⚙ Configurações");
                ui.separator();
                ui.selectable_value(&mut app.page, ActivePage::Usuario, "👤 Meu Perfil");
            });
        });
    }

    fn render_main_content(app: &mut PetMedsApp, ui: &mut egui::Ui) {
        let access_banner = app.current_family.as_ref().and_then(|fam| {
            if fam.access_state != "active" {
                Some((
                    fam.access_state.clone(),
                    fam.starts_on.clone().unwrap_or_default(),
                    fam.ends_on.clone().unwrap_or_default(),
                ))
            } else {
                None
            }
        });

        if let Some((state, starts_on, ends_on)) = access_banner {
            egui::Frame::group(ui.style())
                .inner_margin(16.0)
                .show(ui, |ui| {
                    if state == "scheduled" {
                        ui.heading("⏳ Acesso ainda não iniciado");
                        ui.label(format!(
                            "Seu acesso temporário a esta família começa em {}.",
                            DateUtils::format_br(&starts_on)
                        ));
                    } else {
                        ui.heading("🔒 Acesso desativado");
                        ui.label(format!(
                            "Seu período como cuidador temporário terminou em {}.",
                            DateUtils::format_br(&ends_on)
                        ));
                    }
                });
            if app.page != ActivePage::Config && app.page != ActivePage::Usuario {
                return;
            }
        }

        match app.page {
            ActivePage::Dashboard => Self::render_dashboard(app, ui),
            ActivePage::Pets => Self::render_pets(app, ui),
            ActivePage::Remedios => Self::render_meds(app, ui),
            ActivePage::Tratamentos => Self::render_treatments(app, ui),
            ActivePage::Config => Self::render_config(app, ui),
            ActivePage::Usuario => Self::render_usuario(app, ui),
        }
    }

    // ---------------- 1. DASHBOARD (RF047 - RF050) ----------------

    fn render_dashboard(app: &mut PetMedsApp, ui: &mut egui::Ui) {
        let hoje = DateUtils::today_str();
        let active_pets: Vec<Pet> = app
            .pets
            .iter()
            .filter(|p| p.esta_ativo())
            .cloned()
            .collect();

        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(app.active_pet_id == "all", "🐾 Todos")
                .clicked()
            {
                app.active_pet_id = "all".into();
            }
            for p in &active_pets {
                if ui
                    .selectable_label(
                        app.active_pet_id == p.id,
                        format!("{} {}", p.icone_especie(), p.name),
                    )
                    .clicked()
                {
                    app.active_pet_id = p.id.clone();
                }
            }
        });
        ui.add_space(8.0);

        let selected_pet_ids: std::collections::HashSet<String> = if app.active_pet_id == "all" {
            app.pets.iter().map(|p| p.id.clone()).collect()
        } else {
            [app.active_pet_id.clone()].into_iter().collect()
        };

        let kpi_weight = if app.active_pet_id == "all" {
            let total: f64 = active_pets
                .iter()
                .filter_map(|p| p.ultimo_peso().map(|w| w.weight))
                .sum();
            if active_pets.iter().any(|p| p.ultimo_peso().is_some()) {
                format!("{total:.1} kg")
            } else {
                "-- kg".into()
            }
        } else {
            active_pets
                .iter()
                .find(|p| p.id == app.active_pet_id)
                .and_then(|p| p.ultimo_peso())
                .map(|w| format!("{:.1} kg", w.weight))
                .unwrap_or_else(|| "-- kg".into())
        };

        let week = DateUtils::week_days(app.week_offset);
        let start_w = format!("{}T00:00:00", week[0].0);
        let end_w = format!("{}T23:59:59", week[6].0);

        let visible_treat_ids: std::collections::HashSet<String> = app
            .treatments
            .iter()
            .filter(|t| t.visivel_no_dashboard(&hoje))
            .map(|t| t.id.clone())
            .collect();

        let kpi_doses = app
            .doses
            .iter()
            .filter(|d| {
                selected_pet_ids.contains(&d.pet_id)
                    && d.treatment_id
                        .as_ref()
                        .map(|tid| visible_treat_ids.contains(tid))
                        .unwrap_or(true)
                    && d.scheduled_at >= start_w
                    && d.scheduled_at <= end_w
            })
            .count();

        let kpi_meds = app
            .treatments
            .iter()
            .filter(|t| selected_pet_ids.contains(&t.pet_id) && t.visivel_no_dashboard(&hoje))
            .count();

        ui.columns(3, |cols| {
            cols[0].group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label("Peso Atual");
                    ui.heading(RichText::new(kpi_weight).color(TEAL));
                });
            });
            cols[1].group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label("Doses/Semana");
                    ui.heading(RichText::new(kpi_doses.to_string()).color(TEAL));
                });
            });
            cols[2].group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label("Remédios Ativos");
                    ui.heading(RichText::new(kpi_meds.to_string()).color(TEAL));
                });
            });
        });

        ui.add_space(8.0);
        let fam_state = app
            .current_family
            .as_ref()
            .map(|f| f.access_state.clone())
            .unwrap_or_else(|| "active".to_string());
        let can_act = app.role_policy().pode_registrar_dose(&fam_state);

        if can_act {
            if ui
                .add_sized(
                    [ui.available_width(), 34.0],
                    egui::Button::new(RichText::new("+ Dose Única / Rápida").strong()).fill(TEAL),
                )
                .clicked()
            {
                let first_elig_med_id = app
                    .medications
                    .iter()
                    .find(|m| m.elegivel_dose_unica(&hoje))
                    .map(|m| m.id.clone());

                if active_pets.is_empty() {
                    app.alert(
                        "Nenhum pet ativo",
                        "Cadastre um pet ativo para registrar uma dose.",
                    );
                } else if let Some(med_id) = first_elig_med_id {
                    app.single_pet_id = active_pets[0].id.clone();
                    app.single_med_id = med_id;
                    app.single_actual_at = DateUtils::now_minute_str();
                    app.single_by_user = app
                        .current_user
                        .as_ref()
                        .map(|u| u.id.clone())
                        .unwrap_or_default();
                    app.show_single_dose_modal = true;
                } else {
                    app.alert(
                        "Nenhum medicamento elegível",
                        "Marque um medicamento em estoque e na validade como 'Disponível para dose única'.",
                    );
                }
            }
        }

        ui.add_space(10.0);
        // Carrossel Semanal (RF048)
        egui::Frame::group(ui.style())
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("◀ Semana Anterior").clicked() {
                        app.week_offset -= 1;
                    }
                    ui.label(
                        RichText::new(if app.week_offset == 0 {
                            "Semana Atual".to_string()
                        } else {
                            format!("{:+} Semana(s)", app.week_offset)
                        })
                        .strong(),
                    );
                    if ui.button("Próxima Semana ▶").clicked() {
                        app.week_offset += 1;
                    }
                });
                ui.add_space(6.0);

                if !week.iter().any(|(d, _, _)| d == &app.selected_week_date) {
                    app.selected_week_date = week
                        .iter()
                        .find(|(d, _, _)| d == &hoje)
                        .map(|(d, _, _)| d.clone())
                        .unwrap_or_else(|| week[0].0.clone());
                }

                ui.columns(7, |cols| {
                    for (idx, (d_str, d_name, d_num)) in enumerate(&week) {
                        let is_sel = &app.selected_week_date == d_str;
                        let label = format!("{d_name}\n{d_num}");
                        if cols[idx].selectable_label(is_sel, label).clicked() {
                            app.selected_week_date = d_str.clone();
                        }
                    }
                });
                ui.separator();

                let day_doses: Vec<Dose> = app
                    .doses
                    .iter()
                    .filter(|d| {
                        selected_pet_ids.contains(&d.pet_id)
                            && d.treatment_id
                                .as_ref()
                                .map(|tid| visible_treat_ids.contains(tid))
                                .unwrap_or(true)
                            && d.scheduled_at.starts_with(&app.selected_week_date)
                    })
                    .cloned()
                    .collect();

                if day_doses.is_empty() {
                    ui.label("Nenhuma dose agendada para este dia.");
                } else {
                    for dose in day_doses {
                        let pet_name = app
                            .pets
                            .iter()
                            .find(|p| p.id == dose.pet_id)
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| "Pet".into());
                        let med_name = app
                            .medications
                            .iter()
                            .find(|m| m.id == dose.med_id)
                            .map(|m| m.name.clone())
                            .unwrap_or_else(|| "Medicamento".into());
                        let treat_info = app
                            .treatments
                            .iter()
                            .find(|t| Some(&t.id) == dose.treatment_id.as_ref())
                            .map(|t| (t.schedule_mode.clone(), t.code.clone(), t.dosage.clone()));

                        let time_lbl =
                            if treat_info.as_ref().map(|(m, _, _)| m.as_str()) == Some("all_day") {
                                "Dia todo".to_string()
                            } else {
                                dose.horario_curto().to_string()
                            };
                        let (st_txt, st_rgb) = dose.rotulo_status(&hoje);

                        ui.horizontal(|ui| {
                            Self::badge(ui, st_txt, st_rgb);
                            ui.label(RichText::new(format!("{time_lbl} — {med_name}")).strong());
                            ui.label(format!("({pet_name})"));
                            if let Some((_, t_code, t_dosage)) = &treat_info {
                                ui.label(format!("[Cód: {t_code} | {t_dosage}]"));
                            } else {
                                ui.label("[Dose única]");
                            }

                            if can_act {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if dose.status == "done" || dose.status == "skipped" {
                                            if ui.small_button("↩ Desfazer").clicked() {
                                                let _ = PetMedsController::confirm_dose(
                                                    &app.conn, &dose.id, "pending", None, None,
                                                );
                                                app.refresh_data();
                                            }
                                        } else {
                                            if ui.small_button("❌ Não Dei").clicked() {
                                                let now = DateUtils::now_iso_str();
                                                let _ = PetMedsController::confirm_dose(
                                                    &app.conn,
                                                    &dose.id,
                                                    "skipped",
                                                    Some(&now),
                                                    None,
                                                );
                                                app.refresh_data();
                                            }
                                            if ui.small_button("✅ Dei").clicked() {
                                                app.confirm_dose_id = Some(dose.id.clone());
                                                app.confirm_actual_at = DateUtils::now_minute_str();
                                                app.confirm_by_user = app
                                                    .current_user
                                                    .as_ref()
                                                    .map(|u| u.id.clone())
                                                    .unwrap_or_default();
                                            }
                                        }
                                    },
                                );
                            }
                        });
                        ui.separator();
                    }
                }
            });

        // Histórico de Administrações (RF050)
        ui.add_space(12.0);
        ui.heading("Histórico de Administrações");

        let filter_options: Vec<(String, String)> = if app.history_mode == "person" {
            app.current_family
                .as_ref()
                .map(|fam| {
                    fam.members
                        .iter()
                        .map(|m| (m.user_id.clone(), m.name.clone()))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            app.medications
                .iter()
                .map(|m| (m.id.clone(), m.name.clone()))
                .collect()
        };

        ui.horizontal(|ui| {
            if ui
                .selectable_label(app.history_mode == "person", "Por Pessoa")
                .clicked()
            {
                app.history_mode = "person".into();
                app.history_filter_id = "all".into();
            }
            if ui
                .selectable_label(app.history_mode == "medication", "Por Medicamento")
                .clicked()
            {
                app.history_mode = "medication".into();
                app.history_filter_id = "all".into();
            }
            egui::ComboBox::from_id_salt("hist_filter_combo")
                .selected_text(if app.history_filter_id == "all" {
                    "Todos"
                } else {
                    "Filtrado"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.history_filter_id, "all".into(), "Todos");
                    for (opt_id, opt_name) in &filter_options {
                        ui.selectable_value(&mut app.history_filter_id, opt_id.clone(), opt_name);
                    }
                });
        });

        let mut admin_doses: Vec<Dose> = app
            .doses
            .iter()
            .filter(|d| {
                d.status == "done"
                    && d.administered_by.is_some()
                    && (app.history_filter_id == "all"
                        || (app.history_mode == "person"
                            && d.administered_by.as_deref() == Some(&app.history_filter_id))
                        || (app.history_mode == "medication" && d.med_id == app.history_filter_id))
            })
            .cloned()
            .collect();
        admin_doses.sort_by(|a, b| b.actual_at.cmp(&a.actual_at));

        if admin_doses.is_empty() {
            ui.label("Nenhuma administração registrada para este filtro.");
        } else {
            for d in admin_doses {
                let pet_name = app
                    .pets
                    .iter()
                    .find(|p| p.id == d.pet_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Pet".into());
                let med_name = app
                    .medications
                    .iter()
                    .find(|m| m.id == d.med_id)
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| "Medicamento".into());
                let who = app
                    .current_family
                    .as_ref()
                    .and_then(|f| {
                        f.members
                            .iter()
                            .find(|m| Some(&m.user_id) == d.administered_by.as_ref())
                    })
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| "Membro".into());
                let act_str = d.actual_at.as_deref().unwrap_or(&d.scheduled_at);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        Self::badge(ui, "Administrada", [46, 204, 113]);
                        ui.label(RichText::new(med_name).strong());
                        ui.label(format!(
                            "• Pet: {pet_name} • Por: {who} • Em: {}",
                            DateUtils::format_br(act_str)
                        ));
                    });
                });
            }
        }
    }

    // ---------------- 2. PETS E GRÁFICO DE PESO (RF020 - RF025) ----------------

    fn render_pets(app: &mut PetMedsApp, ui: &mut egui::Ui) {
        let hoje = DateUtils::today_str();
        let can_edit = app.role_policy().pode_editar_cadastros();

        ui.horizontal(|ui| {
            ui.heading("Meus Pets");
            ui.selectable_value(&mut app.pet_filter, "active".into(), "Ativos");
            ui.selectable_value(&mut app.pet_filter, "inactive".into(), "Não ativos");
            if can_edit && ui.button("➕ Cadastrar Pet").clicked() {
                app.pet_id_edit.clear();
                app.pet_name.clear();
                app.pet_species = "Cão".into();
                app.pet_birth = hoje.clone();
                app.pet_status = "active".into();
                app.pet_donated_date.clear();
                app.pet_deceased_date.clear();
                app.show_pet_modal = true;
            }
        });
        ui.separator();

        let list: Vec<Pet> = app
            .pets
            .iter()
            .filter(|p| {
                if app.pet_filter == "active" {
                    p.esta_ativo()
                } else {
                    !p.esta_ativo()
                }
            })
            .cloned()
            .collect();

        if list.is_empty() {
            ui.label("Nenhum pet encontrado nesta aba.");
            return;
        }

        for p in list {
            let active_treats: Vec<String> = app
                .treatments
                .iter()
                .filter(|t| t.pet_id == p.id && t.visivel_no_dashboard(&hoje))
                .map(|t| {
                    let mname = app
                        .medications
                        .iter()
                        .find(|m| m.id == t.med_id)
                        .map(|m| m.name.as_str())
                        .unwrap_or("Med");
                    format!("💊 {} · {}", t.code, mname)
                })
                .collect();

            egui::Frame::group(ui.style())
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (lbl, rgb) = p.rotulo_status(&hoje);
                        Self::badge(ui, lbl, rgb);
                        ui.heading(p.titulo());
                        ui.label(format!("• Nasc.: {}", DateUtils::format_br(&p.birth)));
                        if let Some(w) = p.ultimo_peso() {
                            ui.label(
                                RichText::new(format!(
                                    "⚖ {:.2} kg ({})",
                                    w.weight,
                                    DateUtils::format_br(&w.date)
                                ))
                                .color(TEAL)
                                .strong(),
                            );
                        } else {
                            ui.label("⚖ Peso não registrado");
                        }
                    });

                    if !active_treats.is_empty() {
                        ui.label(format!(
                            "Tratamentos Ativos: {}",
                            join(" | ", active_treats)
                        ));
                    }

                    ui.horizontal(|ui| {
                        if ui.button("📈 Histórico e Gráfico de Peso").clicked() {
                            app.weight_modal_pet_id = Some(p.id.clone());
                            app.weight_record_id.clear();
                            app.weight_val.clear();
                            app.weight_date = hoje.clone();
                        }
                        if can_edit {
                            if ui.button("✏ Editar").clicked() {
                                app.pet_id_edit = p.id.clone();
                                app.pet_name = p.name.clone();
                                app.pet_species = p.species.clone();
                                app.pet_birth = p.birth.clone();
                                app.pet_status = p.status.clone();
                                app.pet_donated_date = p.donated_at.clone().unwrap_or_default();
                                app.pet_deceased_date = p.deceased_at.clone().unwrap_or_default();
                                app.show_pet_modal = true;
                            }
                            if ui.button("🗑 Excluir").clicked() {
                                if let Err(e) = PetMedsDao::delete_pet(&app.conn, &p.id) {
                                    app.alert("Erro ao excluir pet", e);
                                } else {
                                    app.refresh_data();
                                }
                            }
                        }
                    });
                });
            ui.add_space(6.0);
        }
    }

    // ---------------- 3. MEDICAMENTOS E ESTOQUE (RF026 - RF031) ----------------

    fn render_meds(app: &mut PetMedsApp, ui: &mut egui::Ui) {
        let hoje = DateUtils::today_str();
        let can_edit = app.role_policy().pode_editar_cadastros();

        ui.horizontal(|ui| {
            ui.heading("Medicamentos e Estoque");
            ui.selectable_value(
                &mut app.med_filter,
                "available".into(),
                "Com doses disponíveis",
            );
            ui.selectable_value(&mut app.med_filter, "empty".into(), "Sem doses");
            ui.selectable_value(&mut app.med_filter, "expired".into(), "Fora da validade");
            if can_edit && ui.button("➕ Cadastrar Medicamento").clicked() {
                app.med_id_edit.clear();
                app.med_name.clear();
                app.med_lab.clear();
                app.med_qty = "1".into();
                app.med_pkg_qty.clear();
                app.med_single_dose = false;
                app.med_unit = "unidade".into();
                app.med_buy_date = hoje.clone();
                app.med_buy_unknown = false;
                app.med_exp_date = DateUtils::add_days_str(&hoje, 180);
                app.show_med_modal = true;
            }
        });
        ui.separator();

        let list: Vec<Medicamento> = app
            .medications
            .iter()
            .filter(|m| m.categoria_estoque(&hoje) == app.med_filter)
            .cloned()
            .collect();

        if list.is_empty() {
            ui.label("Nenhum medicamento nesta categoria.");
            return;
        }

        for m in list {
            egui::Frame::group(ui.style())
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (lbl, rgb) = m.rotulo_status(&hoje);
                        Self::badge(ui, lbl, rgb);
                        ui.heading(m.titulo());
                        ui.label(format!(
                            "• Estoque: {} {} • Validade: {}",
                            m.qty,
                            m.unit,
                            DateUtils::format_br(&m.exp_date)
                        ));
                    });

                    let mut single = m.single_dose;
                    if ui
                        .add_enabled(
                            can_edit,
                            egui::Checkbox::new(&mut single, "Disponível para dose única"),
                        )
                        .changed()
                    {
                        let _ = PetMedsDao::toggle_single_dose(&app.conn, &m.id, single);
                        app.refresh_data();
                    }

                    ui.collapsing("📦 Histórico de reposições e movimentações", |ui| {
                        for (idx, h) in enumerate(&m.stock_history) {
                            let next_date = m
                                .stock_history
                                .get(idx + 1)
                                .and_then(|nx| nx.purchased_at.as_deref())
                                .unwrap_or(&hoje);
                            let dur = match &h.purchased_at {
                                Some(p) if !p.is_empty() => {
                                    format!("{} dia(s)", DateUtils::days_between(p, next_date))
                                }
                                _ => "Data de compra indefinida".into(),
                            };
                            ui.label(format!(
                                "• [{}] Compra: {} | Qtd: {:+} {} | Validade: {} ({})",
                                h.move_type,
                                h.purchased_at
                                    .as_deref()
                                    .map(DateUtils::format_br)
                                    .unwrap_or_else(|| "indefinida".into()),
                                h.quantity_added,
                                m.unit,
                                DateUtils::format_br(&h.expires_at),
                                dur
                            ));
                        }
                    });

                    if can_edit {
                        ui.horizontal(|ui| {
                            if ui.button("✏ Editar").clicked() {
                                app.med_id_edit = m.id.clone();
                                app.med_name = m.name.clone();
                                app.med_lab = m.lab.clone();
                                app.med_qty = m.qty.to_string();
                                app.med_pkg_qty =
                                    m.package_qty.map(|v| v.to_string()).unwrap_or_default();
                                app.med_single_dose = m.single_dose;
                                app.med_unit = m.unit.clone();
                                app.med_buy_unknown = m.buy_date.is_none();
                                app.med_buy_date = m.buy_date.clone().unwrap_or_default();
                                app.med_exp_date = m.exp_date.clone();
                                app.show_med_modal = true;
                            }
                            if ui.button("➕ Registrar Reposição").clicked() {
                                app.restock_med_id = Some(m.id.clone());
                                app.restock_qty = m
                                    .package_qty
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "1".into());
                                app.restock_buy_date = hoje.clone();
                                app.restock_exp_date = m.exp_date.clone();
                            }
                        });
                    }
                });
            ui.add_space(6.0);
        }
    }

    // ---------------- 4. TRATAMENTOS (RF032 - RF039) ----------------

    fn render_treatments(app: &mut PetMedsApp, ui: &mut egui::Ui) {
        let hoje = DateUtils::today_str();
        let can_edit = app.role_policy().pode_editar_cadastros();

        ui.horizontal(|ui| {
            ui.heading("Tratamentos");
            ui.selectable_value(&mut app.treat_tab, "all".into(), "Todos");
            ui.selectable_value(&mut app.treat_tab, "active".into(), "Ativos");
            ui.selectable_value(&mut app.treat_tab, "paused".into(), "Pausados");
            ui.selectable_value(&mut app.treat_tab, "completed".into(), "Concluídos");

            if can_edit && ui.button("➕ Novo Tratamento").clicked() {
                if app.pets.is_empty() || app.medications.is_empty() {
                    app.alert(
                        "Cadastros necessários",
                        "Cadastre ao menos um Pet e um Medicamento antes de criar um tratamento.",
                    );
                } else {
                    app.treat_id_edit.clear();
                    app.treat_pet_id = app.pets[0].id.clone();
                    app.treat_med_id = app.medications[0].id.clone();
                    app.treat_dosage_qty = "1".into();
                    app.treat_dosage_unit = "comprimido(s)".into();
                    app.treat_schedule_mode = "exact".into();
                    app.treat_tolerance = "30".into();
                    app.treat_start_date = hoje.clone();
                    app.treat_interval_val = "1".into();
                    app.treat_interval_unit = "day".into();
                    app.treat_duration_type = "indefinite".into();
                    app.treat_end_date.clear();
                    app.treat_times = vec![("08:00".into(), true)];
                    app.show_treat_modal = true;
                }
            }
        });
        ui.separator();

        let list: Vec<Tratamento> = app
            .treatments
            .iter()
            .filter(|t| app.treat_tab == "all" || t.categoria(&hoje) == app.treat_tab)
            .cloned()
            .collect();

        if list.is_empty() {
            ui.label("Nenhum tratamento encontrado para esta aba.");
            return;
        }

        for t in list {
            let pet_name = app
                .pets
                .iter()
                .find(|p| p.id == t.pet_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "Pet".into());
            let med_name = app
                .medications
                .iter()
                .find(|m| m.id == t.med_id)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| "Medicamento".into());
            let given = app
                .doses
                .iter()
                .filter(|d| d.treatment_id.as_deref() == Some(&t.id) && d.status == "done")
                .count();
            let planned = PetMedsController::planned_dose_count(&t);

            egui::Frame::group(ui.style())
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        Self::badge(ui, &format!("Cód: {}", t.code), [28, 151, 146]);
                        ui.heading(format!("{pet_name} — {med_name}"));
                        let cat = t.categoria(&hoje);
                        if cat == "paused" {
                            Self::badge(ui, "Pausado", [230, 126, 34]);
                        } else if cat == "completed" {
                            Self::badge(ui, "Concluído", [100, 116, 139]);
                        }
                    });
                    ui.label(format!(
                        "Posologia: {} | Modo: {} | A cada {} {} | Início: {}",
                        t.dosage,
                        t.schedule_mode,
                        t.interval_value,
                        t.interval_unit,
                        DateUtils::format_br(&t.start_date)
                    ));
                    ui.label(match planned {
                        Some(p) => format!("Progresso: Administradas {given} de {p} planejadas"),
                        None => format!("Progresso: Administradas {given} (Duração indefinida)"),
                    });

                    for p in &t.pause_history {
                        ui.small(format!(
                            "⏸ Pausa: {} até {}",
                            p.paused_at,
                            p.resumed_at.as_deref().unwrap_or("em andamento")
                        ));
                    }

                    if can_edit {
                        ui.horizontal(|ui| {
                            if !t.esta_concluido(&hoje) {
                                if t.paused_at.is_some() {
                                    if ui.button("▶ Retomar").clicked() {
                                        let _ =
                                            PetMedsController::resume_treatment(&app.conn, &t.id);
                                        app.refresh_data();
                                    }
                                } else if ui.button("⏸ Pausar").clicked() {
                                    let _ = PetMedsController::pause_treatment(&app.conn, &t.id);
                                    app.refresh_data();
                                }
                                if ui.button("⏹ Encerrar agora").clicked() {
                                    let _ = PetMedsController::finish_treatment(&app.conn, &t.id);
                                    app.refresh_data();
                                }
                            }
                            if ui.button("✏ Editar").clicked() {
                                app.treat_id_edit = t.id.clone();
                                app.treat_pet_id = t.pet_id.clone();
                                app.treat_med_id = t.med_id.clone();
                                app.treat_dosage_qty = t.dosage_quantity.to_string();
                                app.treat_dosage_unit = t.dosage_unit.clone();
                                app.treat_schedule_mode = t.schedule_mode.clone();
                                app.treat_tolerance = t.tolerance_minutes.to_string();
                                app.treat_start_date = t.start_date.clone();
                                app.treat_interval_val = t.interval_value.to_string();
                                app.treat_interval_unit = t.interval_unit.clone();
                                app.treat_duration_type = t.duration_type.clone();
                                app.treat_end_date = t.end_date.clone().unwrap_or_default();
                                app.treat_times = t
                                    .times
                                    .iter()
                                    .map(|tm| (tm.clone(), !t.disabled_times.contains(tm)))
                                    .collect();
                                app.show_treat_modal = true;
                            }
                            if ui.button("🗑 Excluir").clicked() {
                                let _ = PetMedsController::delete_treatment(&app.conn, &t.id);
                                app.refresh_data();
                            }
                        });
                    }
                });
            ui.add_space(6.0);
        }
    }

    // ---------------- 5. CONFIGURAÇÕES E FAMÍLIA (RF009 - RF019, RF051, RF052) ----------------

    fn render_config(app: &mut PetMedsApp, ui: &mut egui::Ui) {
        let is_admin = app.role_policy().pode_gerenciar_familia();
        let my_uid = app
            .current_user
            .as_ref()
            .map(|u| u.id.clone())
            .unwrap_or_default();

        if let Some(fam) = app.current_family.clone() {
            egui::Frame::group(ui.style())
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.heading(format!("Família Ativa: {}", fam.name));
                    if is_admin {
                        ui.label(
                            RichText::new(format!(
                                "🔑 Código de Convite (6 dígitos): {}",
                                fam.code
                            ))
                            .color(TEAL)
                            .strong(),
                        );
                    }
                    ui.separator();
                    ui.label(RichText::new("Membros Participantes:").strong());
                    for m in &fam.members {
                        ui.horizontal(|ui| {
                            ui.label(format!("👤 {} — Papel: {}", m.name, m.role));
                            if m.role == "cuidador_temporario" {
                                ui.small(format!(
                                    "(Período: {} a {})",
                                    m.starts_on.as_deref().unwrap_or("livre"),
                                    m.ends_on.as_deref().unwrap_or("livre")
                                ));
                            }
                            if is_admin && m.user_id != my_uid {
                                if ui.small_button("Remover").clicked() {
                                    let _ =
                                        PetMedsDao::remove_member(&app.conn, &fam.id, &m.user_id);
                                    app.refresh_data();
                                }
                            }
                        });
                    }

                    if is_admin {
                        ui.separator();
                        ui.label(RichText::new("Convidar / Atualizar Membro por E-mail:").strong());
                        ui.horizontal(|ui| {
                            ui.label("E-mail:");
                            ui.text_edit_singleline(&mut app.invite_email);
                            egui::ComboBox::from_id_salt("inv_role")
                                .selected_text(&app.invite_role)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut app.invite_role,
                                        "cuidador".into(),
                                        "Cuidador",
                                    );
                                    ui.selectable_value(
                                        &mut app.invite_role,
                                        "cuidador_temporario".into(),
                                        "Cuidador temporário",
                                    );
                                    ui.selectable_value(
                                        &mut app.invite_role,
                                        "visualizador".into(),
                                        "Visualizador",
                                    );
                                    ui.selectable_value(
                                        &mut app.invite_role,
                                        "admin".into(),
                                        "Admin",
                                    );
                                });
                        });
                        if app.invite_role == "cuidador_temporario" {
                            ui.horizontal(|ui| {
                                ui.label("Início (AAAA-MM-DD):");
                                ui.text_edit_singleline(&mut app.invite_starts);
                                ui.label("Fim (AAAA-MM-DD):");
                                ui.text_edit_singleline(&mut app.invite_ends);
                            });
                        }
                        if ui.button("Adicionar / Atualizar Membro").clicked() {
                            match PetMedsDao::invite_member(
                                &app.conn,
                                &fam.id,
                                &app.invite_email,
                                &app.invite_role,
                                Some(app.invite_starts.clone()),
                                Some(app.invite_ends.clone()),
                            ) {
                                Ok(_) => {
                                    app.invite_email.clear();
                                    app.refresh_data();
                                    app.alert("Sucesso", "Membro vinculado à família.");
                                }
                                Err(e) => app.alert("Erro ao convidar", e),
                            }
                        }

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut app.rename_family_name);
                            if ui.button("Renomear Família").clicked() {
                                let _ = PetMedsDao::rename_family(
                                    &app.conn,
                                    &fam.id,
                                    &app.rename_family_name,
                                );
                                app.refresh_data();
                            }
                            if ui.button("🗑 Excluir Família").clicked() {
                                if let Some(u) = app.current_user.clone() {
                                    let _ = PetMedsDao::delete_family(&app.conn, &fam.id, &u);
                                    app.refresh_data();
                                }
                            }
                        });
                    }
                });
        }

        ui.add_space(10.0);
        ui.columns(2, |cols| {
            cols[0].group(|ui| {
                ui.heading("Criar Nova Família");
                ui.text_edit_singleline(&mut app.new_family_name);
                if ui.button("Criar Família").clicked() && !my_uid.is_empty() {
                    match PetMedsDao::create_family(&app.conn, &my_uid, &app.new_family_name) {
                        Ok(_) => {
                            app.new_family_name.clear();
                            app.refresh_data();
                        }
                        Err(e) => app.alert("Erro", e),
                    }
                }
            });
            cols[1].group(|ui| {
                ui.heading("Entrar com Código (6 dígitos)");
                ui.text_edit_singleline(&mut app.join_family_code);
                if ui.button("Ingressar como Cuidador").clicked() && !my_uid.is_empty() {
                    match PetMedsDao::join_family_by_code(&app.conn, &my_uid, &app.join_family_code)
                    {
                        Ok(_) => {
                            app.join_family_code.clear();
                            app.refresh_data();
                        }
                        Err(e) => app.alert("Código inválido", e),
                    }
                }
            });
        });

        ui.add_space(10.0);
        egui::Frame::group(ui.style())
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Preferências e Backup");
                    if ui.checkbox(&mut app.dark_mode, "🌙 Tema Escuro").changed() {
                        PetMedsApp::apply_theme(ui.ctx(), app.dark_mode);
                        PetMedsDao::set_pref(
                            &app.conn,
                            "theme",
                            if app.dark_mode { "dark" } else { "light" },
                        );
                    }
                    if ui.button("💾 Exportar Backup JSON").clicked() {
                        if let Some(fam) = app.current_family.clone() {
                            match PetMedsController::export_family_json(
                                &fam,
                                &app.pets,
                                &app.medications,
                                &app.treatments,
                                &app.doses,
                            ) {
                                Ok(path) => app
                                    .alert("Backup Concluído", format!("Arquivo salvo em: {path}")),
                                Err(e) => app.alert("Erro ao exportar", e),
                            }
                        }
                    }
                });
            });
    }

    // ---------------- 6. PERFIL DO USUÁRIO (RF004 - RF008) ----------------

    fn render_usuario(app: &mut PetMedsApp, ui: &mut egui::Ui) {
        let my_uid = app.current_user.as_ref().map(|u| u.id.clone());

        egui::Frame::group(ui.style())
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.heading("Meu Perfil");
                ui.label("Nome Completo:");
                ui.text_edit_singleline(&mut app.prof_name);
                ui.label("E-mail:");
                ui.text_edit_singleline(&mut app.prof_email);
                ui.label("Nova Senha (deixe em branco para manter a atual):");
                ui.add(egui::TextEdit::singleline(&mut app.prof_pass).password(true));
                ui.checkbox(&mut app.prof_notif, "Receber lembretes de doses por E-mail");

                ui.horizontal(|ui| {
                    if ui.button("💾 Atualizar Perfil").clicked() {
                        if let Some(uid) = &my_uid {
                            match PetMedsDao::update_profile(
                                &app.conn,
                                uid,
                                &app.prof_name,
                                &app.prof_email,
                                &app.prof_pass,
                                app.prof_notif,
                            ) {
                                Ok(_) => {
                                    app.prof_pass.clear();
                                    app.current_user = PetMedsDao::load_user_by_id(&app.conn, uid);
                                    app.alert("Perfil atualizado", "Seus dados foram salvos.");
                                }
                                Err(e) => app.alert("Erro", e),
                            }
                        }
                    }
                    if ui.button("🔔 Testar Notificação Desktop").clicked() {
                        app.alert(
                            "PetMeds Desktop",
                            "Notificações nativas de área de trabalho estão ativas!",
                        );
                    }
                    if ui.button("🚪 Sair (Logout)").clicked() {
                        let _ = app
                            .conn
                            .execute("DELETE FROM app_prefs WHERE key = 'session_user_id'", []);
                        app.current_user = None;
                        app.current_family = None;
                    }
                    if ui.button("🗑 Excluir Minha Conta").clicked() {
                        if let Some(uid) = &my_uid {
                            let _ = PetMedsDao::delete_account(&app.conn, uid);
                            app.current_user = None;
                            app.current_family = None;
                        }
                    }
                });
            });
    }

    // ---------------- MODAIS E GRÁFICO DE LINHA EM CANVAS NATIVO ----------------

    fn render_modals(app: &mut PetMedsApp, ctx: &egui::Context) {
        let fam_id_opt = app.current_family.as_ref().map(|f| f.id.clone());
        let fam_members: Vec<(String, String)> = app
            .current_family
            .as_ref()
            .map(|f| {
                f.members
                    .iter()
                    .map(|m| (m.user_id.clone(), m.name.clone()))
                    .collect()
            })
            .unwrap_or_default();

        // Modal Cadastrar/Editar Pet
        if app.show_pet_modal {
            egui::Window::new("🐾 Cadastro de Pet")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Nome do Pet:");
                    ui.text_edit_singleline(&mut app.pet_name);
                    ui.label("Espécie:");
                    egui::ComboBox::from_id_salt("pet_sp_cb")
                        .selected_text(&app.pet_species)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.pet_species, "Cão".into(), "Cão");
                            ui.selectable_value(&mut app.pet_species, "Gato".into(), "Gato");
                            ui.selectable_value(&mut app.pet_species, "Outro".into(), "Outro");
                        });
                    ui.label("Data de Nascimento (AAAA-MM-DD):");
                    ui.text_edit_singleline(&mut app.pet_birth);
                    ui.label("Status:");
                    egui::ComboBox::from_id_salt("pet_st_cb")
                        .selected_text(&app.pet_status)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.pet_status, "active".into(), "Ativo");
                            ui.selectable_value(&mut app.pet_status, "donated".into(), "Doado");
                            ui.selectable_value(&mut app.pet_status, "lost".into(), "Perdido");
                            ui.selectable_value(&mut app.pet_status, "deceased".into(), "Falecido");
                        });
                    if app.pet_status == "donated" {
                        ui.label("Data da doação (AAAA-MM-DD):");
                        ui.text_edit_singleline(&mut app.pet_donated_date);
                    }
                    if app.pet_status == "deceased" {
                        ui.label("Data do falecimento (AAAA-MM-DD):");
                        ui.text_edit_singleline(&mut app.pet_deceased_date);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Salvar Pet").clicked() {
                            if let Some(fam_id) = &fam_id_opt {
                                match PetMedsDao::save_pet(
                                    &app.conn,
                                    Some(&app.pet_id_edit),
                                    fam_id,
                                    &app.pet_name,
                                    &app.pet_species,
                                    &app.pet_birth,
                                    &app.pet_status,
                                    Some(app.pet_donated_date.clone()),
                                    Some(app.pet_deceased_date.clone()),
                                ) {
                                    Ok(_) => {
                                        app.show_pet_modal = false;
                                        app.refresh_data();
                                    }
                                    Err(e) => app.alert("Não foi possível salvar o pet", e),
                                }
                            }
                        }
                        if ui.button("Cancelar").clicked() {
                            app.show_pet_modal = false;
                        }
                    });
                });
        }

        // Modal Histórico e Gráfico de Peso (RF024 & RF025)
        if let Some(pet_id) = app.weight_modal_pet_id.clone() {
            if let Some(pet) = app.pets.iter().find(|p| p.id == pet_id).cloned() {
                let can_edit = app.role_policy().pode_editar_cadastros();
                egui::Window::new(format!("⚖ Evolução de Peso — {}", pet.name))
                    .collapsible(false)
                    .show(ctx, |ui| {
                        if can_edit {
                            ui.horizontal(|ui| {
                                ui.label("Peso (kg):");
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.weight_val)
                                        .desired_width(70.0),
                                );
                                ui.label("Data:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.weight_date)
                                        .desired_width(100.0),
                                );
                                if ui.button("Salvar Peso").clicked() {
                                    let val = app
                                        .weight_val
                                        .replace(',', ".")
                                        .parse::<f64>()
                                        .unwrap_or(0.0);
                                    match PetMedsDao::save_weight(
                                        &app.conn,
                                        Some(&app.weight_record_id),
                                        &pet.id,
                                        val,
                                        &app.weight_date,
                                    ) {
                                        Ok(_) => {
                                            app.weight_record_id.clear();
                                            app.weight_val.clear();
                                            app.refresh_data();
                                        }
                                        Err(e) => app.alert("Erro", e),
                                    }
                                }
                            });
                            ui.separator();
                        }

                        if !pet.weight_history.is_empty() {
                            let (rect, _) = ui
                                .allocate_exact_size(Vec2::new(380.0, 160.0), egui::Sense::hover());
                            let painter = ui.painter_at(rect);
                            painter.rect_filled(
                                rect,
                                8.0,
                                Color32::from_rgba_unmultiplied(28, 151, 146, 22),
                            );

                            let vals: Vec<f64> =
                                pet.weight_history.iter().map(|w| w.weight).collect();
                            let min_v = min(vals.clone()).unwrap_or(0.0);
                            let max_v = max(vals).unwrap_or(1.0);
                            let span = (max_v - min_v).max(0.5);

                            let pad = 24.0;
                            let w_area = rect.width() - pad * 2.0;
                            let h_area = rect.height() - pad * 2.0;
                            let n = len(&pet.weight_history);

                            let mut pts = Vec::with_capacity(n);
                            for (i, w) in enumerate(&pet.weight_history) {
                                let x = rect.left()
                                    + pad
                                    + if n <= 1 {
                                        w_area / 2.0
                                    } else {
                                        (i as f32 / (n - 1) as f32) * w_area
                                    };
                                let y = rect.bottom()
                                    - pad
                                    - (((w.weight - min_v) / span) as f32 * h_area);
                                pts.append_item(Pos2::new(x, y));
                            }
                            for pair in pts.windows(2) {
                                painter.line_segment([pair[0], pair[1]], Stroke::new(2.5, TEAL));
                            }
                            for p_pos in pts {
                                painter.circle_filled(p_pos, 4.5, TEAL);
                            }
                        } else {
                            ui.label("Nenhum peso registrado.");
                        }

                        for w in &pet.weight_history {
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "{} — {:.2} kg",
                                    DateUtils::format_br(&w.date),
                                    w.weight
                                ));
                                if can_edit && ui.small_button("Editar").clicked() {
                                    app.weight_record_id = w.id.clone();
                                    app.weight_val = w.weight.to_string();
                                    app.weight_date = w.date.clone();
                                }
                            });
                        }
                        if ui.button("Fechar").clicked() {
                            app.weight_modal_pet_id = None;
                        }
                    });
            }
        }

        // Modal Medicamento
        if app.show_med_modal {
            egui::Window::new("💊 Cadastro de Medicamento")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Nome:");
                    ui.text_edit_singleline(&mut app.med_name);
                    ui.label("Laboratório:");
                    ui.text_edit_singleline(&mut app.med_lab);
                    ui.horizontal(|ui| {
                        ui.label("Qtd Estoque:");
                        ui.add(egui::TextEdit::singleline(&mut app.med_qty).desired_width(70.0));
                        egui::ComboBox::from_id_salt("med_unit_cb")
                            .selected_text(&app.med_unit)
                            .show_ui(ui, |ui| {
                                for u in ["unidade", "dose", "ml", "mg"] {
                                    ui.selectable_value(&mut app.med_unit, u.into(), u);
                                }
                            });
                    });
                    ui.label("Quantidade por embalagem (opcional):");
                    ui.text_edit_singleline(&mut app.med_pkg_qty);
                    ui.checkbox(&mut app.med_single_dose, "Disponível para dose única");
                    ui.checkbox(&mut app.med_buy_unknown, "Data de compra indefinida");
                    if !app.med_buy_unknown {
                        ui.label("Data da Compra (AAAA-MM-DD):");
                        ui.text_edit_singleline(&mut app.med_buy_date);
                    }
                    ui.label("Data de Validade (AAAA-MM-DD):");
                    ui.text_edit_singleline(&mut app.med_exp_date);

                    ui.horizontal(|ui| {
                        if ui.button("Salvar").clicked() {
                            if let Some(fam_id) = &fam_id_opt {
                                let q =
                                    app.med_qty.replace(',', ".").parse::<f64>().unwrap_or(-1.0);
                                let pq = app.med_pkg_qty.replace(',', ".").parse::<f64>().ok();
                                let bdate = if app.med_buy_unknown {
                                    None
                                } else {
                                    Some(app.med_buy_date.clone())
                                };
                                match PetMedsDao::save_medication(
                                    &app.conn,
                                    Some(&app.med_id_edit),
                                    fam_id,
                                    &app.med_name,
                                    &app.med_lab,
                                    q,
                                    pq,
                                    app.med_single_dose,
                                    &app.med_unit,
                                    bdate,
                                    &app.med_exp_date,
                                ) {
                                    Ok(_) => {
                                        app.show_med_modal = false;
                                        app.refresh_data();
                                    }
                                    Err(e) => app.alert("Erro", e),
                                }
                            }
                        }
                        if ui.button("Cancelar").clicked() {
                            app.show_med_modal = false;
                        }
                    });
                });
        }

        // Modal Reposição de Estoque
        if let Some(mid) = app.restock_med_id.clone() {
            egui::Window::new("➕ Reposição de Estoque")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Quantidade adicionada:");
                    ui.text_edit_singleline(&mut app.restock_qty);
                    ui.label("Data da compra (AAAA-MM-DD):");
                    ui.text_edit_singleline(&mut app.restock_buy_date);
                    ui.label("Validade da nova caixa (AAAA-MM-DD):");
                    ui.text_edit_singleline(&mut app.restock_exp_date);
                    ui.horizontal(|ui| {
                        if ui.button("Confirmar Reposição").clicked() {
                            let q = app
                                .restock_qty
                                .replace(',', ".")
                                .parse::<f64>()
                                .unwrap_or(0.0);
                            match PetMedsDao::restock_medication(
                                &app.conn,
                                &mid,
                                q,
                                &app.restock_buy_date,
                                &app.restock_exp_date,
                            ) {
                                Ok(_) => {
                                    app.restock_med_id = None;
                                    app.refresh_data();
                                }
                                Err(e) => app.alert("Erro", e),
                            }
                        }
                        if ui.button("Cancelar").clicked() {
                            app.restock_med_id = None;
                        }
                    });
                });
        }

        // Modal Tratamento
        if app.show_treat_modal {
            let pet_options: Vec<(String, String)> = app
                .pets
                .iter()
                .map(|p| (p.id.clone(), p.name.clone()))
                .collect();
            let med_options: Vec<(String, String)> = app
                .medications
                .iter()
                .map(|m| (m.id.clone(), m.name.clone()))
                .collect();

            egui::Window::new("📋 Configurar Tratamento")
                .collapsible(false)
                .show(ctx, |ui| {
                    egui::ComboBox::from_label("Pet")
                        .selected_text(&app.treat_pet_id)
                        .show_ui(ui, |ui| {
                            for (pid, pname) in &pet_options {
                                ui.selectable_value(&mut app.treat_pet_id, pid.clone(), pname);
                            }
                        });
                    egui::ComboBox::from_label("Medicamento")
                        .selected_text(&app.treat_med_id)
                        .show_ui(ui, |ui| {
                            for (mid, mname) in &med_options {
                                ui.selectable_value(&mut app.treat_med_id, mid.clone(), mname);
                            }
                        });
                    ui.horizontal(|ui| {
                        ui.label("Posologia:");
                        ui.add(
                            egui::TextEdit::singleline(&mut app.treat_dosage_qty)
                                .desired_width(60.0),
                        );
                        egui::ComboBox::from_id_salt("dos_u_cb")
                            .selected_text(&app.treat_dosage_unit)
                            .show_ui(ui, |ui| {
                                for u in [
                                    "comprimido(s)",
                                    "cápsula(s)",
                                    "ml",
                                    "mg",
                                    "gota(s)",
                                    "dose(s)",
                                ] {
                                    ui.selectable_value(&mut app.treat_dosage_unit, u.into(), u);
                                }
                            });
                    });
                    egui::ComboBox::from_label("Modo de Horário")
                        .selected_text(&app.treat_schedule_mode)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.treat_schedule_mode,
                                "exact".into(),
                                "Horário específico",
                            );
                            ui.selectable_value(
                                &mut app.treat_schedule_mode,
                                "window".into(),
                                "Janela de tolerância",
                            );
                            ui.selectable_value(
                                &mut app.treat_schedule_mode,
                                "all_day".into(),
                                "Dia inteiro",
                            );
                        });

                    if app.treat_schedule_mode == "window" {
                        ui.label("Tolerância (minutos):");
                        ui.text_edit_singleline(&mut app.treat_tolerance);
                    }
                    if app.treat_schedule_mode != "all_day" {
                        ui.label("Horários (HH:MM):");
                        let mut remove_idx = None;
                        for (idx, (t_str, en)) in app.treat_times.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.checkbox(en, "Ativo");
                                ui.add(egui::TextEdit::singleline(t_str).desired_width(70.0));
                                if idx > 0 && ui.small_button("✖").clicked() {
                                    remove_idx = Some(idx);
                                }
                            });
                        }
                        if let Some(i) = remove_idx {
                            app.treat_times.remove(i);
                        }
                        if ui.small_button("+ Adicionar Horário").clicked() {
                            app.treat_times.append_item(("20:00".into(), true));
                        }
                    }

                    ui.label("Data Inicial (AAAA-MM-DD):");
                    ui.text_edit_singleline(&mut app.treat_start_date);
                    ui.horizontal(|ui| {
                        ui.label("Repetir a cada:");
                        ui.add(
                            egui::TextEdit::singleline(&mut app.treat_interval_val)
                                .desired_width(50.0),
                        );
                        egui::ComboBox::from_id_salt("int_u_cb")
                            .selected_text(&app.treat_interval_unit)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut app.treat_interval_unit,
                                    "day".into(),
                                    "dia(s)",
                                );
                                ui.selectable_value(
                                    &mut app.treat_interval_unit,
                                    "week".into(),
                                    "semana(s)",
                                );
                                ui.selectable_value(
                                    &mut app.treat_interval_unit,
                                    "month".into(),
                                    "mês(es)",
                                );
                                ui.selectable_value(
                                    &mut app.treat_interval_unit,
                                    "year".into(),
                                    "ano(s)",
                                );
                            });
                    });
                    egui::ComboBox::from_label("Duração")
                        .selected_text(&app.treat_duration_type)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.treat_duration_type,
                                "indefinite".into(),
                                "Indefinido",
                            );
                            ui.selectable_value(
                                &mut app.treat_duration_type,
                                "until_date".into(),
                                "Até data final",
                            );
                        });
                    if app.treat_duration_type == "until_date" {
                        ui.label("Data Final (AAAA-MM-DD):");
                        ui.text_edit_singleline(&mut app.treat_end_date);
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Salvar Tratamento").clicked() {
                            if let Some(fam_id) = &fam_id_opt {
                                let d_qty = app
                                    .treat_dosage_qty
                                    .replace(',', ".")
                                    .parse::<f64>()
                                    .unwrap_or(0.0);
                                let tol = app.treat_tolerance.parse::<i32>().unwrap_or(30);
                                let iv = app.treat_interval_val.parse::<i32>().unwrap_or(1);
                                match PetMedsController::save_treatment(
                                    &app.conn,
                                    Some(&app.treat_id_edit),
                                    fam_id,
                                    &app.treat_pet_id,
                                    &app.treat_med_id,
                                    d_qty,
                                    &app.treat_dosage_unit,
                                    &app.treat_schedule_mode,
                                    tol,
                                    &app.treat_start_date,
                                    iv,
                                    &app.treat_interval_unit,
                                    &app.treat_duration_type,
                                    Some(app.treat_end_date.clone()),
                                    &app.treat_times,
                                ) {
                                    Ok(_) => {
                                        app.show_treat_modal = false;
                                        app.refresh_data();
                                    }
                                    Err(e) => app.alert("Erro ao salvar tratamento", e),
                                }
                            }
                        }
                        if ui.button("Cancelar").clicked() {
                            app.show_treat_modal = false;
                        }
                    });
                });
        }

        // Modal Confirmar Dose ("Dei")
        if let Some(did) = app.confirm_dose_id.clone() {
            egui::Window::new("✅ Confirmar Administração")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Data e horário real (AAAA-MM-DDTHH:MM):");
                    ui.text_edit_singleline(&mut app.confirm_actual_at);
                    egui::ComboBox::from_label("Responsável")
                        .selected_text(&app.confirm_by_user)
                        .show_ui(ui, |ui| {
                            for (uid, uname) in &fam_members {
                                ui.selectable_value(&mut app.confirm_by_user, uid.clone(), uname);
                            }
                        });
                    ui.horizontal(|ui| {
                        if ui.button("Confirmar").clicked() {
                            match PetMedsController::confirm_dose(
                                &app.conn,
                                &did,
                                "done",
                                Some(&app.confirm_actual_at),
                                Some(&app.confirm_by_user),
                            ) {
                                Ok(_) => {
                                    app.confirm_dose_id = None;
                                    app.refresh_data();
                                }
                                Err(e) => app.alert("Erro", e),
                            }
                        }
                        if ui.button("Cancelar").clicked() {
                            app.confirm_dose_id = None;
                        }
                    });
                });
        }

        // Modal Dose Única (RF046)
        if app.show_single_dose_modal {
            let hoje = DateUtils::today_str();
            let active_pet_opts: Vec<(String, String)> = app
                .pets
                .iter()
                .filter(|p| p.esta_ativo())
                .map(|p| (p.id.clone(), p.name.clone()))
                .collect();
            let elig_med_opts: Vec<(String, String)> = app
                .medications
                .iter()
                .filter(|m| m.elegivel_dose_unica(&hoje))
                .map(|m| (m.id.clone(), m.name.clone()))
                .collect();

            egui::Window::new("⚡ Registrar Dose Única")
                .collapsible(false)
                .show(ctx, |ui| {
                    egui::ComboBox::from_label("Pet Ativo")
                        .selected_text(&app.single_pet_id)
                        .show_ui(ui, |ui| {
                            for (pid, pname) in &active_pet_opts {
                                ui.selectable_value(&mut app.single_pet_id, pid.clone(), pname);
                            }
                        });
                    egui::ComboBox::from_label("Medicamento Elegível")
                        .selected_text(&app.single_med_id)
                        .show_ui(ui, |ui| {
                            for (mid, mname) in &elig_med_opts {
                                ui.selectable_value(&mut app.single_med_id, mid.clone(), mname);
                            }
                        });
                    ui.label("Data/Hora (AAAA-MM-DDTHH:MM):");
                    ui.text_edit_singleline(&mut app.single_actual_at);
                    egui::ComboBox::from_label("Responsável")
                        .selected_text(&app.single_by_user)
                        .show_ui(ui, |ui| {
                            for (uid, uname) in &fam_members {
                                ui.selectable_value(&mut app.single_by_user, uid.clone(), uname);
                            }
                        });
                    ui.horizontal(|ui| {
                        if ui.button("Registrar Dose Única").clicked() {
                            if let Some(fam_id) = &fam_id_opt {
                                match PetMedsController::register_single_dose(
                                    &app.conn,
                                    fam_id,
                                    &app.single_pet_id,
                                    &app.single_med_id,
                                    &app.single_actual_at,
                                    &app.single_by_user,
                                ) {
                                    Ok(_) => {
                                        app.show_single_dose_modal = false;
                                        app.refresh_data();
                                    }
                                    Err(e) => app.alert("Erro", e),
                                }
                            }
                        }
                        if ui.button("Cancelar").clicked() {
                            app.show_single_dose_modal = false;
                        }
                    });
                });
        }
    }

    fn render_alert_modal(app: &mut PetMedsApp, ctx: &egui::Context) {
        if let Some((title, msg)) = app.modal_alert.clone() {
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(&msg);
                    ui.add_space(8.0);
                    if ui.button("OK").clicked() {
                        app.modal_alert = None;
                    }
                });
        }
    }
}
