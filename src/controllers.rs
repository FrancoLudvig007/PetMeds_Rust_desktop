// src/controllers.rs
use crate::dao::PetMedsDao;
use crate::models::*;
use crate::pyrsutils::prelude::*;
use rusqlite::{params, Connection};

pub struct PetMedsController;

impl PetMedsController {
    /// RN29: Verifica se um horário cai dentro de qualquer período de pausa do tratamento
    pub fn is_within_pause(tr: &Tratamento, scheduled_iso: &str) -> bool {
        tr.pause_history.iter().any(|p| {
            let after_start = scheduled_iso >= p.paused_at.as_str();
            let before_end = p
                .resumed_at
                .as_deref()
                .map(|end| scheduled_iso < end)
                .unwrap_or(true);
            after_start && before_end
        })
    }

    /// RN26: Verifica se a data-alvo coincide com um dia de administração do tratamento
    pub fn is_treatment_date_due(tr: &Tratamento, target_ymd: &str) -> bool {
        if target_ymd < tr.start_date.as_str() {
            return false;
        }
        for occ in range(1001) {
            let occ_date = DateUtils::add_treatment_interval(
                &tr.start_date,
                &tr.interval_unit,
                tr.interval_value,
                occ as i32,
            );
            if occ_date.as_str() > target_ymd {
                return false;
            }
            if occ_date == target_ymd {
                return true;
            }
        }
        false
    }

    /// RN32: Calcula o total de doses planejadas quando o tratamento possui data final definida
    pub fn planned_dose_count(tr: &Tratamento) -> Option<usize> {
        let end_date = tr.end_date.as_deref()?.trim();
        if end_date.is_empty() {
            return None;
        }
        let mut count = 0usize;
        for occ in range(1001) {
            let occ_date = DateUtils::add_treatment_interval(
                &tr.start_date,
                &tr.interval_unit,
                tr.interval_value,
                occ as i32,
            );
            if occ_date.as_str() > end_date {
                break;
            }
            for t_str in &tr.times {
                if tr.disabled_times.contains(t_str) {
                    continue;
                }
                let sched = format!("{occ_date}T{t_str}:00");
                if let Some(fin) = &tr.finished_at {
                    if sched > *fin {
                        continue;
                    }
                }
                if !Self::is_within_pause(tr, &sched) {
                    count += 1;
                }
            }
        }
        Some(count)
    }

    /// RF040 & RF041: Gera doses para janela móvel de 30 dias e atualiza status para 'late'
    pub fn sync_scheduled_and_overdue_doses(conn: &Connection, fam_id: &str) -> Result<(), String> {
        let treatments = PetMedsDao::load_treatments(conn, fam_id);
        let today = DateUtils::today_str();
        let now_iso = DateUtils::now_iso_str();

        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        for tr in &treatments {
            if tr.paused_at.is_some() || tr.finished_at.is_some() {
                continue;
            }
            for day_idx in range(30) {
                let target_date = DateUtils::add_days_str(&today, day_idx);
                if let Some(end_d) = &tr.end_date {
                    if !end_d.is_empty() && target_date > *end_d {
                        continue;
                    }
                }
                if !Self::is_treatment_date_due(tr, &target_date) {
                    continue;
                }
                for time_s in &tr.times {
                    if tr.disabled_times.contains(time_s) {
                        continue;
                    }
                    let sched_iso = format!("{target_date}T{time_s}:00");
                    if Self::is_within_pause(tr, &sched_iso) {
                        continue;
                    }
                    let _ = tx.execute(
                        "INSERT OR IGNORE INTO doses
                         (id, family_id, treatment_id, pet_id, medication_id, scheduled_at, status, notified)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', 0)",
                        params![
                            DateUtils::new_uuid(),
                            fam_id,
                            tr.id,
                            tr.pet_id,
                            tr.med_id,
                            sched_iso
                        ],
                    );
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;

        // Marca doses pendentes que ultrapassaram o prazo como 'late' (RN36)
        let doses = PetMedsDao::load_doses(conn, fam_id);
        let tx_late = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        for d in &doses {
            if d.status != "pending" {
                continue;
            }
            let tr = treatments
                .iter()
                .find(|t| Some(&t.id) == d.treatment_id.as_ref());
            let is_overdue = match tr.map(|t| t.schedule_mode.as_str()) {
                Some("all_day") => d.data_curta() < today.as_str(),
                Some("window") => {
                    let tol = tr.map(|t| t.tolerance_minutes).unwrap_or(0);
                    let limit = DateUtils::add_minutes_iso(&d.scheduled_at, tol);
                    limit < now_iso
                }
                _ => d.scheduled_at < now_iso,
            };
            if is_overdue {
                let _ = tx_late.execute(
                    "UPDATE doses SET status = 'late' WHERE id = ?1",
                    params![d.id],
                );
            }
        }
        tx_late.commit().map_err(|e| e.to_string())
    }

    pub fn save_treatment(
        conn: &Connection,
        id: Option<&str>,
        fam_id: &str,
        pet_id: &str,
        med_id: &str,
        dosage_qty: f64,
        dosage_unit: &str,
        schedule_mode: &str,
        tolerance_minutes: i32,
        start_date: &str,
        interval_value: i32,
        interval_unit: &str,
        duration_type: &str,
        end_date: Option<String>,
        times: &[(String, bool)],
    ) -> Result<(), String> {
        if pet_id.is_empty() || med_id.is_empty() || dosage_qty <= 0.0 || start_date.is_empty() {
            return Err("Preencha o pet, medicamento, posologia (> 0) e data inicial.".into());
        }
        if interval_value < 1 {
            return Err("O intervalo deve ser um inteiro maior ou igual a 1.".into());
        }
        if schedule_mode == "window" && tolerance_minutes < 1 {
            return Err("Informe ao menos 1 minuto de tolerância no modo janela.".into());
        }
        let end_clean = if duration_type == "until_date" {
            let e = end_date.unwrap_or_default();
            if strip(&e).is_empty() {
                return Err("Informe a data final do tratamento.".into());
            }
            Some(e)
        } else {
            None
        };

        let final_times: Vec<(String, bool)> = if schedule_mode == "all_day" {
            vec![("12:00".to_string(), true)]
        } else {
            let valid: Vec<(String, bool)> = times
                .iter()
                .filter(|(t, _)| !strip(t).is_empty())
                .cloned()
                .collect();
            let mut uniq = std::collections::HashSet::new();
            for (t, _) in &valid {
                if !uniq.insert(t.clone()) {
                    return Err(
                        "Não são permitidos horários duplicados no mesmo tratamento.".into(),
                    );
                }
            }
            if !valid.iter().any(|(_, en)| *en) {
                return Err(
                    "Mantenha pelo menos um horário ativo ou selecione Dia inteiro.".into(),
                );
            }
            valid
        };

        let dosage_str = format!("{dosage_qty} {dosage_unit}");
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

        let tid = if let Some(existing_id) = id.filter(|s| !s.is_empty()) {
            // RN30: Remove doses pendentes e atrasadas e preserva histórico de realizadas
            tx.execute(
                "DELETE FROM doses WHERE treatment_id = ?1 AND status IN ('pending', 'late')",
                params![existing_id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "UPDATE treatments SET pet_id = ?1, medication_id = ?2, dosage = ?3,
                 dosage_quantity = ?4, dosage_unit = ?5, schedule_mode = ?6, tolerance_minutes = ?7,
                 start_date = ?8, interval_value = ?9, interval_unit = ?10, duration_type = ?11,
                 end_date = ?12 WHERE id = ?13",
                params![
                    pet_id,
                    med_id,
                    dosage_str,
                    dosage_qty,
                    dosage_unit,
                    schedule_mode,
                    tolerance_minutes,
                    start_date,
                    interval_value,
                    interval_unit,
                    duration_type,
                    end_clean,
                    existing_id
                ],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "DELETE FROM treatment_times WHERE treatment_id = ?1",
                params![existing_id],
            )
            .map_err(|e| e.to_string())?;
            existing_id.to_string()
        } else {
            let new_id = DateUtils::new_uuid();
            tx.execute(
                "INSERT INTO treatments (id, family_id, pet_id, medication_id, dosage,
                 dosage_quantity, dosage_unit, schedule_mode, tolerance_minutes, start_date,
                 interval_value, interval_unit, duration_type, end_date, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    new_id,
                    fam_id,
                    pet_id,
                    med_id,
                    dosage_str,
                    dosage_qty,
                    dosage_unit,
                    schedule_mode,
                    tolerance_minutes,
                    start_date,
                    interval_value,
                    interval_unit,
                    duration_type,
                    end_clean,
                    DateUtils::now_iso_str()
                ],
            )
            .map_err(|e| e.to_string())?;
            new_id
        };

        for (t_str, en) in final_times {
            tx.execute(
                "INSERT INTO treatment_times (treatment_id, time_str, enabled) VALUES (?1, ?2, ?3)",
                params![tid, t_str, i32::from(en)],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn pause_treatment(conn: &Connection, tid: &str) -> Result<(), String> {
        let now = DateUtils::now_iso_str();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE treatments SET paused_at = ?1 WHERE id = ?2",
            params![now, tid],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO pause_history (id, treatment_id, paused_at) VALUES (?1, ?2, ?3)",
            params![DateUtils::new_uuid(), tid, now],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM doses WHERE treatment_id = ?1 AND status IN ('pending', 'late') AND scheduled_at >= ?2",
            params![tid, now],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn resume_treatment(conn: &Connection, tid: &str) -> Result<(), String> {
        let now = DateUtils::now_iso_str();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE treatments SET paused_at = NULL WHERE id = ?1",
            params![tid],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE pause_history SET resumed_at = ?1 WHERE treatment_id = ?2 AND resumed_at IS NULL",
            params![now, tid],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn finish_treatment(conn: &Connection, tid: &str) -> Result<(), String> {
        let now = DateUtils::now_iso_str();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE treatments SET finished_at = ?1, paused_at = NULL WHERE id = ?2",
            params![now, tid],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE pause_history SET resumed_at = ?1 WHERE treatment_id = ?2 AND resumed_at IS NULL",
            params![now, tid],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM doses WHERE treatment_id = ?1 AND status IN ('pending', 'late') AND scheduled_at >= ?2",
            params![tid, now],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn delete_treatment(conn: &Connection, tid: &str) -> Result<(), String> {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM doses WHERE treatment_id = ?1", params![tid])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM treatments WHERE id = ?1", params![tid])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    // ---------------- Administração de Doses (Módulo F) ----------------

    pub fn confirm_dose(
        conn: &Connection,
        dose_id: &str,
        status: &str,
        actual_at: Option<&str>,
        administered_by: Option<&str>,
    ) -> Result<(), String> {
        if status == "done" {
            if actual_at.map(|s| strip(s).is_empty()).unwrap_or(true) {
                return Err("Informe data e horário da administração.".into());
            }
            if administered_by.map(|s| strip(s).is_empty()).unwrap_or(true) {
                return Err("Selecione o membro responsável pela administração.".into());
            }
        }
        let act = if status == "pending" { None } else { actual_at };
        let resp = if status == "done" {
            administered_by
        } else {
            None
        };
        conn.execute(
            "UPDATE doses SET status = ?1, actual_at = ?2, administered_by = ?3, notified = 0 WHERE id = ?4",
            params![status, act, resp, dose_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn register_single_dose(
        conn: &Connection,
        fam_id: &str,
        pet_id: &str,
        med_id: &str,
        actual_at: &str,
        administered_by: &str,
    ) -> Result<(), String> {
        if pet_id.is_empty()
            || med_id.is_empty()
            || strip(actual_at).is_empty()
            || administered_by.is_empty()
        {
            return Err("Preencha o pet, medicamento, data/horário e responsável.".into());
        }
        let iso = if len(actual_at) == 16 {
            format!("{actual_at}:00")
        } else {
            actual_at.to_string()
        };
        conn.execute(
            "INSERT INTO doses (id, family_id, treatment_id, pet_id, medication_id, scheduled_at, actual_at, administered_by, status, notified)
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?5, ?6, 'done', 1)",
            params![DateUtils::new_uuid(), fam_id, pet_id, med_id, iso, administered_by],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// RF008: Verifica a cada minuto se há doses pendentes no minuto exato e retorna alertas
    pub fn check_due_notifications(
        conn: &Connection,
        doses: &[Dose],
        pets: &[Pet],
        meds: &[Medicamento],
    ) -> Vec<String> {
        let current_min = DateUtils::now_minute_str();
        let mut alerts = Vec::new();
        for d in doses {
            if d.status == "pending" && !d.notified && d.scheduled_at.starts_with(&current_min) {
                let pet_name = pets
                    .iter()
                    .find(|p| p.id == d.pet_id)
                    .map(|p| p.name.as_str())
                    .unwrap_or("Pet");
                let med_name = meds
                    .iter()
                    .find(|m| m.id == d.med_id)
                    .map(|m| m.name.as_str())
                    .unwrap_or("Medicamento");
                alerts.append_item(format!(
                    "🔔 Lembrete de Dose ({}) — Hora de administrar {} para {}!",
                    d.horario_curto(),
                    med_name,
                    pet_name
                ));
                let _ = conn.execute("UPDATE doses SET notified = 1 WHERE id = ?1", params![d.id]);
            }
        }
        alerts
    }

    /// RF052: Exporta os dados da família ativa em arquivo JSON legível usando `pyrsutils::fs_utils`
    pub fn export_family_json(
        fam: &Familia,
        pets: &[Pet],
        meds: &[Medicamento],
        treatments: &[Tratamento],
        doses: &[Dose],
    ) -> Result<String, String> {
        let filename = format!("petmeds_backup_{}.json", DateUtils::today_str());
        let pets_json: Vec<String> = pets
            .iter()
            .map(|p| {
                format!(
                    "    {{\"id\": \"{}\", \"name\": \"{}\", \"species\": \"{}\", \"birth\": \"{}\", \"status\": \"{}\"}}",
                    p.id, p.name, p.species, p.birth, p.status
                )
            })
            .collect();
        let meds_json: Vec<String> = meds
            .iter()
            .map(|m| {
                format!(
                    "    {{\"id\": \"{}\", \"name\": \"{}\", \"lab\": \"{}\", \"qty\": {}, \"unit\": \"{}\", \"expires_at\": \"{}\"}}",
                    m.id, m.name, m.lab, m.qty, m.unit, m.exp_date
                )
            })
            .collect();
        let treats_json: Vec<String> = treatments
            .iter()
            .map(|t| {
                format!(
                    "    {{\"id\": \"{}\", \"code\": \"{}\", \"pet_id\": \"{}\", \"med_id\": \"{}\", \"dosage\": \"{}\", \"start_date\": \"{}\"}}",
                    t.id, t.code, t.pet_id, t.med_id, t.dosage, t.start_date
                )
            })
            .collect();
        let doses_json: Vec<String> = doses
            .iter()
            .map(|d| {
                format!(
                    "    {{\"id\": \"{}\", \"pet_id\": \"{}\", \"med_id\": \"{}\", \"scheduled_at\": \"{}\", \"status\": \"{}\"}}",
                    d.id, d.pet_id, d.med_id, d.scheduled_at, d.status
                )
            })
            .collect();

        let json_content = format!(
            "{{\n  \"family\": {{\"id\": \"{}\", \"name\": \"{}\", \"code\": \"{}\"}},\n  \"pets\": [\n{}\n  ],\n  \"medications\": [\n{}\n  ],\n  \"treatments\": [\n{}\n  ],\n  \"doses\": [\n{}\n  ]\n}}",
            fam.id,
            fam.name,
            fam.code,
            join(",\n", pets_json),
            join(",\n", meds_json),
            join(",\n", treats_json),
            join(",\n", doses_json)
        );

        write_text(&filename, json_content).map_err(|e| e.to_string())?;
        Ok(filename)
    }
}
