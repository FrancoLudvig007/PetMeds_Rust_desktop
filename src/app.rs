// src/app.rs
use crate::controllers::PetMedsController;
use crate::dao::PetMedsDao;
use crate::db::Database;
use crate::models::*;
use crate::pyrsutils::prelude::*;
use crate::views::PetMedsViews;
use eframe::egui;
use rusqlite::Connection;

#[derive(PartialEq, Clone, Copy)]
pub enum ActivePage {
    Dashboard,
    Pets,
    Remedios,
    Tratamentos,
    Config,
    Usuario,
}

#[derive(PartialEq, Clone, Copy)]
pub enum AuthViewMode {
    Login,
    Register,
    Forgot,
}

pub struct PetMedsApp {
    pub conn: Connection,
    pub current_user: Option<Usuario>,
    pub current_family: Option<Familia>,
    pub user_families: Vec<(String, String, String)>,
    pub pets: Vec<Pet>,
    pub medications: Vec<Medicamento>,
    pub treatments: Vec<Tratamento>,
    pub doses: Vec<Dose>,

    // Navegação e filtros de tela
    pub page: ActivePage,
    pub auth_mode: AuthViewMode,
    pub dark_mode: bool,
    pub active_pet_id: String,
    pub pet_filter: String,
    pub med_filter: String,
    pub treat_tab: String,
    pub week_offset: i64,
    pub selected_week_date: String,
    pub history_mode: String,
    pub history_filter_id: String,

    // Campos de Autenticação e Perfil
    pub auth_name: String,
    pub auth_email: String,
    pub auth_pass: String,
    pub auth_confirm: String,
    pub prof_name: String,
    pub prof_email: String,
    pub prof_pass: String,
    pub prof_notif: bool,

    // Campos de Família
    pub new_family_name: String,
    pub join_family_code: String,
    pub rename_family_name: String,
    pub invite_email: String,
    pub invite_role: String,
    pub invite_starts: String,
    pub invite_ends: String,

    // Modal Pet e Peso
    pub show_pet_modal: bool,
    pub pet_id_edit: String,
    pub pet_name: String,
    pub pet_species: String,
    pub pet_birth: String,
    pub pet_status: String,
    pub pet_donated_date: String,
    pub pet_deceased_date: String,

    pub weight_modal_pet_id: Option<String>,
    pub weight_record_id: String,
    pub weight_val: String,
    pub weight_date: String,

    // Modal Medicamento e Reposição
    pub show_med_modal: bool,
    pub med_id_edit: String,
    pub med_name: String,
    pub med_lab: String,
    pub med_qty: String,
    pub med_pkg_qty: String,
    pub med_single_dose: bool,
    pub med_unit: String,
    pub med_buy_date: String,
    pub med_buy_unknown: bool,
    pub med_exp_date: String,

    pub restock_med_id: Option<String>,
    pub restock_qty: String,
    pub restock_buy_date: String,
    pub restock_exp_date: String,

    // Modal Tratamento
    pub show_treat_modal: bool,
    pub treat_id_edit: String,
    pub treat_pet_id: String,
    pub treat_med_id: String,
    pub treat_dosage_qty: String,
    pub treat_dosage_unit: String,
    pub treat_schedule_mode: String,
    pub treat_tolerance: String,
    pub treat_start_date: String,
    pub treat_interval_val: String,
    pub treat_interval_unit: String,
    pub treat_duration_type: String,
    pub treat_end_date: String,
    pub treat_times: Vec<(String, bool)>,

    // Modais de Dose
    pub confirm_dose_id: Option<String>,
    pub confirm_actual_at: String,
    pub confirm_by_user: String,
    pub show_single_dose_modal: bool,
    pub single_pet_id: String,
    pub single_med_id: String,
    pub single_actual_at: String,
    pub single_by_user: String,

    // Alertas e controle de verificação de notificações
    pub modal_alert: Option<(String, String)>,
    pub last_notif_check: f64,
}

impl PetMedsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let conn = Database::connect().expect("Falha ao inicializar banco SQLite local");
        let saved_theme = PetMedsDao::get_pref(&conn, "theme").unwrap_or_else(|| "dark".into());
        let dark_mode = saved_theme == "dark";
        Self::apply_theme(&cc.egui_ctx, dark_mode);

        let today = DateUtils::today_str();
        let mut app = Self {
            conn,
            current_user: None,
            current_family: None,
            user_families: Vec::new(),
            pets: Vec::new(),
            medications: Vec::new(),
            treatments: Vec::new(),
            doses: Vec::new(),
            page: ActivePage::Dashboard,
            auth_mode: AuthViewMode::Login,
            dark_mode,
            active_pet_id: "all".into(),
            pet_filter: "active".into(),
            med_filter: "available".into(),
            treat_tab: "active".into(),
            week_offset: 0,
            selected_week_date: today.clone(),
            history_mode: "person".into(),
            history_filter_id: "all".into(),
            auth_name: String::new(),
            auth_email: String::new(),
            auth_pass: String::new(),
            auth_confirm: String::new(),
            prof_name: String::new(),
            prof_email: String::new(),
            prof_pass: String::new(),
            prof_notif: true,
            new_family_name: String::new(),
            join_family_code: String::new(),
            rename_family_name: String::new(),
            invite_email: String::new(),
            invite_role: "cuidador".into(),
            invite_starts: String::new(),
            invite_ends: String::new(),
            show_pet_modal: false,
            pet_id_edit: String::new(),
            pet_name: String::new(),
            pet_species: "Cão".into(),
            pet_birth: today.clone(),
            pet_status: "active".into(),
            pet_donated_date: String::new(),
            pet_deceased_date: String::new(),
            weight_modal_pet_id: None,
            weight_record_id: String::new(),
            weight_val: String::new(),
            weight_date: today.clone(),
            show_med_modal: false,
            med_id_edit: String::new(),
            med_name: String::new(),
            med_lab: String::new(),
            med_qty: "1".into(),
            med_pkg_qty: String::new(),
            med_single_dose: false,
            med_unit: "unidade".into(),
            med_buy_date: today.clone(),
            med_buy_unknown: false,
            med_exp_date: DateUtils::add_days_str(&today, 180),
            restock_med_id: None,
            restock_qty: String::new(),
            restock_buy_date: today.clone(),
            restock_exp_date: DateUtils::add_days_str(&today, 180),
            show_treat_modal: false,
            treat_id_edit: String::new(),
            treat_pet_id: String::new(),
            treat_med_id: String::new(),
            treat_dosage_qty: "1".into(),
            treat_dosage_unit: "comprimido(s)".into(),
            treat_schedule_mode: "exact".into(),
            treat_tolerance: "30".into(),
            treat_start_date: today,
            treat_interval_val: "1".into(),
            treat_interval_unit: "day".into(),
            treat_duration_type: "indefinite".into(),
            treat_end_date: String::new(),
            treat_times: vec![("08:00".into(), true)],
            confirm_dose_id: None,
            confirm_actual_at: DateUtils::now_minute_str(),
            confirm_by_user: String::new(),
            show_single_dose_modal: false,
            single_pet_id: String::new(),
            single_med_id: String::new(),
            single_actual_at: DateUtils::now_minute_str(),
            single_by_user: String::new(),
            modal_alert: None,
            last_notif_check: 0.0,
        };

        if let Some(uid) = PetMedsDao::get_pref(&app.conn, "session_user_id") {
            if let Some(u) = PetMedsDao::load_user_by_id(&app.conn, &uid) {
                app.current_user = Some(u);
                app.refresh_data();
            }
        }
        app
    }

    pub fn apply_theme(ctx: &egui::Context, dark: bool) {
        let mut visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(10.0);
        visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
        visuals.widgets.active.rounding = egui::Rounding::same(8.0);
        ctx.set_visuals(visuals);
    }

    pub fn role_policy(&self) -> PapelUsuario {
        let role = self
            .current_family
            .as_ref()
            .and_then(|f| {
                let uid = &self.current_user.as_ref()?.id;
                f.members
                    .iter()
                    .find(|m| &m.user_id == uid)
                    .map(|m| m.role.clone())
            })
            .unwrap_or_else(|| "visualizador".into());
        PapelUsuario(role)
    }

    pub fn refresh_data(&mut self) {
        let Some(user) = self.current_user.clone() else {
            return;
        };
        self.prof_name = user.name.clone();
        self.prof_email = user.email.clone();
        self.prof_notif = user.receive_email_notif;

        self.user_families = PetMedsDao::list_user_families(&self.conn, &user.id);
        if self.user_families.is_empty() {
            self.current_family = None;
            return;
        }
        let saved_fid = PetMedsDao::get_pref(&self.conn, "active_family_id").unwrap_or_default();
        let active_fid = self
            .user_families
            .iter()
            .find(|(id, _, _)| id == &saved_fid)
            .map(|(id, _, _)| id.clone())
            .unwrap_or_else(|| self.user_families[0].0.clone());
        PetMedsDao::set_pref(&self.conn, "active_family_id", &active_fid);

        self.current_family = PetMedsDao::load_family(&self.conn, &active_fid, &user.id);
        if let Some(fam) = &self.current_family {
            self.rename_family_name = fam.name.clone();
            if fam.access_state != "active" {
                self.pets.clear();
                self.medications.clear();
                self.treatments.clear();
                self.doses.clear();
                return;
            }
            let _ = PetMedsController::sync_scheduled_and_overdue_doses(&self.conn, &fam.id);
            self.pets = PetMedsDao::load_pets(&self.conn, &fam.id);
            self.medications = PetMedsDao::load_medications(&self.conn, &fam.id);
            self.treatments = PetMedsDao::load_treatments(&self.conn, &fam.id);
            self.doses = PetMedsDao::load_doses(&self.conn, &fam.id);
        }
    }

    pub fn alert(&mut self, title: &str, msg: impl Into<String>) {
        self.modal_alert = Some((title.to_string(), msg.into()));
    }
}

impl eframe::App for PetMedsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now_sec = perf_counter();
        if self.current_user.is_some()
            && (now_sec - self.last_notif_check >= 60.0 || self.last_notif_check == 0.0)
        {
            self.last_notif_check = now_sec;
            let alerts = PetMedsController::check_due_notifications(
                &self.conn,
                &self.doses,
                &self.pets,
                &self.medications,
            );
            if let Some(first_alert) = alerts.first() {
                self.alert("Notificação de Dose", first_alert.clone());
                self.refresh_data();
            }
        }

        PetMedsViews::render(self, ctx);
        ctx.request_repaint_after(std::time::Duration::from_secs(30));
    }
}
