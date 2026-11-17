// src/models.rs
use crate::pyrsutils::prelude::*;

// ============================================================================
// Polimorfismo: Interfaces Comportamentais para UI e Controle de Acesso (RBAC)
// ============================================================================

pub trait CardPolimorfico {
    fn titulo(&self) -> String;
    fn rotulo_status(&self, hoje: &str) -> (&'static str, [u8; 3]);
}

pub trait PoliticaAcesso {
    fn nome_papel(&self) -> &'static str;
    fn pode_gerenciar_familia(&self) -> bool;
    fn pode_editar_cadastros(&self) -> bool;
    fn pode_registrar_dose(&self, estado_acesso: &str) -> bool;
}

pub struct PapelUsuario(pub String);

impl PoliticaAcesso for PapelUsuario {
    fn nome_papel(&self) -> &'static str {
        match self.0.as_str() {
            "admin" => "Admin",
            "cuidador" => "Cuidador",
            "cuidador_temporario" => "Cuidador temporário",
            "visualizador" => "Visualizador",
            _ => "Membro",
        }
    }
    fn pode_gerenciar_familia(&self) -> bool {
        self.0 == "admin"
    }
    fn pode_editar_cadastros(&self) -> bool {
        self.0 == "admin" || self.0 == "cuidador"
    }
    fn pode_registrar_dose(&self, estado_acesso: &str) -> bool {
        estado_acesso == "active"
            && (self.0 == "admin" || self.0 == "cuidador" || self.0 == "cuidador_temporario")
    }
}

// ============================================================================
// Herança POO com `class!` da pyrsutils.rs
// ============================================================================

class! {
    struct EntidadeBase {
        id: String,
        family_id: String,
    }
    impl {
        fn pertence_a(&self, fam_id: &str) -> bool {
            self.family_id == fam_id
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegistroPeso {
    pub id: String,
    pub weight: f64,
    pub date: String,
}

class! {
    struct Pet : EntidadeBase {
        name: String,
        species: String,
        birth: String,
        status: String,
        donated_at: Option<String>,
        deceased_at: Option<String>,
        photo: Option<String>,
        weight_history: Vec<RegistroPeso>,
    }
    impl {
        fn esta_ativo(&self) -> bool {
            self.status == "active"
        }
        fn ultimo_peso(&self) -> Option<&RegistroPeso> {
            self.weight_history.last()
        }
        fn icone_especie(&self) -> &'static str {
            match self.species.as_str() {
                "Gato" => "🐱",
                "Cão" => "🐶",
                _ => "🐾",
            }
        }
    }
}

impl CardPolimorfico for Pet {
    fn titulo(&self) -> String {
        format!("{} {}", self.icone_especie(), self.name)
    }
    fn rotulo_status(&self, _hoje: &str) -> (&'static str, [u8; 3]) {
        match self.status.as_str() {
            "active" => ("Ativo", [46, 204, 113]),
            "donated" => ("Doado", [230, 126, 34]),
            "lost" => ("Perdido", [230, 126, 34]),
            "deceased" => ("Falecido", [231, 76, 60]),
            _ => ("Ativo", [46, 204, 113]),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MovimentacaoEstoque {
    pub move_type: String,
    pub purchased_at: Option<String>,
    pub expires_at: String,
    pub quantity_added: f64,
    pub recorded_at: String,
}

class! {
    struct Medicamento : EntidadeBase {
        name: String,
        lab: String,
        qty: f64,
        package_qty: Option<f64>,
        single_dose: bool,
        unit: String,
        buy_date: Option<String>,
        exp_date: String,
        stock_history: Vec<MovimentacaoEstoque>,
    }
    impl {
        fn categoria_estoque(&self, hoje: &str) -> &'static str {
            if !self.exp_date.is_empty() && self.exp_date.as_str() < hoje {
                "expired"
            } else if self.qty > 0.0 {
                "available"
            } else {
                "empty"
            }
        }
        fn elegivel_dose_unica(&self, hoje: &str) -> bool {
            self.single_dose && self.categoria_estoque(hoje) == "available"
        }
    }
}

impl CardPolimorfico for Medicamento {
    fn titulo(&self) -> String {
        format!("💊 {} ({})", self.name, self.lab)
    }
    fn rotulo_status(&self, hoje: &str) -> (&'static str, [u8; 3]) {
        let em_30_dias = DateUtils::add_days_str(hoje, 30);
        if self.categoria_estoque(hoje) == "expired" {
            ("FORA DA VALIDADE", [231, 76, 60])
        } else if self.exp_date <= em_30_dias {
            ("VENCE EM BREVE", [230, 126, 34])
        } else if self.qty <= 0.0 {
            ("SEM DOSES", [148, 163, 184])
        } else {
            ("DISPONÍVEL", [46, 204, 113])
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PeriodoPausa {
    pub id: String,
    pub paused_at: String,
    pub resumed_at: Option<String>,
}

class! {
    struct Tratamento : EntidadeBase {
        code: String,
        pet_id: String,
        med_id: String,
        dosage: String,
        dosage_quantity: f64,
        dosage_unit: String,
        schedule_mode: String,
        tolerance_minutes: i32,
        start_date: String,
        interval_value: i32,
        interval_unit: String,
        duration_type: String,
        end_date: Option<String>,
        paused_at: Option<String>,
        finished_at: Option<String>,
        created_at: String,
        times: Vec<String>,
        disabled_times: Vec<String>,
        pause_history: Vec<PeriodoPausa>,
    }
    impl {
        fn esta_concluido(&self, hoje: &str) -> bool {
            self.finished_at.is_some()
                || self.end_date.as_deref().map(|d| d < hoje).unwrap_or(false)
        }
        fn categoria(&self, hoje: &str) -> &'static str {
            if self.paused_at.is_some() && !self.esta_concluido(hoje) {
                "paused"
            } else if self.esta_concluido(hoje) {
                "completed"
            } else {
                "active"
            }
        }
        fn visivel_no_dashboard(&self, hoje: &str) -> bool {
            self.paused_at.is_none() && !self.esta_concluido(hoje)
        }
    }
}

class! {
    struct Dose : EntidadeBase {
        treatment_id: Option<String>,
        pet_id: String,
        med_id: String,
        scheduled_at: String,
        actual_at: Option<String>,
        administered_by: Option<String>,
        status: String,
        notified: bool,
    }
    impl {
        fn horario_curto(&self) -> &str {
            if len(&self.scheduled_at) >= 16 {
                &self.scheduled_at[11..16]
            } else {
                "--:--"
            }
        }
        fn data_curta(&self) -> &str {
            if len(&self.scheduled_at) >= 10 {
                &self.scheduled_at[0..10]
            } else {
                ""
            }
        }
    }
}

impl CardPolimorfico for Dose {
    fn titulo(&self) -> String {
        format!("Dose {}", self.horario_curto())
    }
    fn rotulo_status(&self, _hoje: &str) -> (&'static str, [u8; 3]) {
        match self.status.as_str() {
            "done" => ("Administrada", [46, 204, 113]),
            "late" => ("Atrasada", [231, 76, 60]),
            "skipped" => ("Não dada", [230, 126, 34]),
            _ => ("Pendente", [100, 116, 139]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Usuario {
    pub id: String,
    pub name: String,
    pub email: String,
    pub receive_email_notif: bool,
    pub photo: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MembroFamilia {
    pub user_id: String,
    pub name: String,
    pub role: String,
    pub receive_email: bool,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Familia {
    pub id: String,
    pub name: String,
    pub code: String,
    pub admin_id: String,
    pub access_state: String,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub members: Vec<MembroFamilia>,
}

// ============================================================================
// Utilitários de Data/Hora e Geração de IDs baseados na pyrsutils.rs
// ============================================================================

pub struct DateUtils;

impl DateUtils {
    pub fn new_uuid() -> String {
        let mut bytes = [0u8; 16];
        for b in bytes.iter_mut() {
            *b = randint(0, 255) as u8;
        }
        let hex = hex_encode(&bytes);
        format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        )
    }

    pub fn new_family_code() -> String {
        let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
        let mut code = String::with_capacity(6);
        for _ in range(6) {
            if let Some(&c) = choice(&chars) {
                code.push(c);
            }
        }
        code
    }

    pub fn today_str() -> String {
        let t = gmtime_now();
        strftime("%Y-%m-%d", &t)
    }

    pub fn now_minute_str() -> String {
        let t = gmtime_now();
        strftime("%Y-%m-%dT%H:%M", &t)
    }

    pub fn now_iso_str() -> String {
        let t = gmtime_now();
        strftime("%Y-%m-%dT%H:%M:%S", &t)
    }

    pub fn format_br(date_str: &str) -> String {
        if len(date_str) >= 10 {
            let partes = split(&date_str[0..10], "-");
            if len(&partes) == 3 {
                return format!("{}/{}/{}", partes[2], partes[1], partes[0]);
            }
        }
        date_str.to_string()
    }

    pub fn parse_ymd(date_str: &str) -> Option<(i64, u32, u32)> {
        if len(date_str) < 10 {
            return None;
        }
        let p = split(&date_str[0..10], "-");
        if len(&p) != 3 {
            return None;
        }
        Some((p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?))
    }

    pub fn to_epoch_days(date_str: &str) -> Option<i64> {
        let (y, m, d) = Self::parse_ymd(date_str)?;
        let st = StructTime {
            year: y,
            month: m,
            day: d,
            hour: 12,
            minute: 0,
            second: 0,
            weekday: 0,
            yearday: 1,
        };
        Some(mktime_utc(&st).div_euclid(86_400))
    }

    pub fn add_days_str(date_str: &str, delta_days: i64) -> String {
        if let Some(days) = Self::to_epoch_days(date_str) {
            let st = gmtime((days + delta_days) * 86_400 + 43_200);
            strftime("%Y-%m-%d", &st)
        } else {
            date_str.to_string()
        }
    }

    pub fn days_between(d1: &str, d2: &str) -> i64 {
        match (Self::to_epoch_days(d1), Self::to_epoch_days(d2)) {
            (Some(a), Some(b)) => (b - a).max(0),
            _ => 0,
        }
    }

    pub fn last_day_of_month(year: i64, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                if is_leap {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }

    pub fn add_treatment_interval(start_ymd: &str, unit: &str, val: i32, count: i32) -> String {
        let step = (val * count) as i64;
        match unit {
            "day" => Self::add_days_str(start_ymd, step),
            "week" => Self::add_days_str(start_ymd, step * 7),
            "month" => {
                if let Some((y, m, d)) = Self::parse_ymd(start_ymd) {
                    let total_months = (y * 12 + (m as i64 - 1)) + step;
                    let ny = total_months.div_euclid(12);
                    let nm = (total_months.rem_euclid(12) + 1) as u32;
                    let max_d = Self::last_day_of_month(ny, nm);
                    format!("{:04}-{:02}-{:02}", ny, nm, d.min(max_d))
                } else {
                    start_ymd.to_string()
                }
            }
            "year" => {
                if let Some((y, m, d)) = Self::parse_ymd(start_ymd) {
                    let ny = y + step;
                    let max_d = Self::last_day_of_month(ny, m);
                    format!("{:04}-{:02}-{:02}", ny, m, d.min(max_d))
                } else {
                    start_ymd.to_string()
                }
            }
            _ => Self::add_days_str(start_ymd, step),
        }
    }

    pub fn week_days(offset_weeks: i64) -> Vec<(String, &'static str, u32)> {
        let now = gmtime_now();
        let today_str = strftime("%Y-%m-%d", &now);
        let days_since_sunday = ((now.weekday + 1) % 7) as i64;
        let sunday_str = Self::add_days_str(&today_str, -days_since_sunday + offset_weeks * 7);
        let names = ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"];
        let mut out = Vec::with_capacity(7);
        for i in range(7) {
            let d_str = Self::add_days_str(&sunday_str, i);
            let day_num = Self::parse_ymd(&d_str).map(|(_, _, d)| d).unwrap_or(1);
            out.append_item((d_str, names[i as usize], day_num));
        }
        out
    }

    pub fn add_minutes_iso(iso_dt: &str, minutes: i32) -> String {
        if len(iso_dt) >= 16 {
            if let Some((y, m, d)) = Self::parse_ymd(&iso_dt[0..10]) {
                let hm = split(&iso_dt[11..16], ":");
                if len(&hm) == 2 {
                    let h: u32 = hm[0].parse().unwrap_or(0);
                    let min: u32 = hm[1].parse().unwrap_or(0);
                    let st = StructTime {
                        year: y,
                        month: m,
                        day: d,
                        hour: h,
                        minute: min,
                        second: 0,
                        weekday: 0,
                        yearday: 1,
                    };
                    let epoch = mktime_utc(&st) + (minutes as i64 * 60);
                    return strftime("%Y-%m-%dT%H:%M:%S", &gmtime(epoch));
                }
            }
        }
        iso_dt.to_string()
    }
}
