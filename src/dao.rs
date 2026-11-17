// src/dao.rs
use crate::models::*;
use crate::pyrsutils::prelude::*;
use rusqlite::{params, Connection, OptionalExtension};

pub struct PetMedsDao;

impl PetMedsDao {
    pub fn get_pref(conn: &Connection, key: &str) -> Option<String> {
        conn.query_row(
            "SELECT val FROM app_prefs WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn set_pref(conn: &Connection, key: &str, val: &str) {
        let _ = conn.execute(
            "INSERT INTO app_prefs (key, val) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET val = excluded.val",
            params![key, val],
        );
    }

    // ---------------- Autenticação e Conta (Módulo A) ----------------

    pub fn register_user(
        conn: &Connection,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<Usuario, String> {
        let email_clean = lower(strip(email));
        let name_clean = strip(name);
        if name_clean.is_empty() || email_clean.is_empty() || password.is_empty() {
            return Err("Preencha nome completo, e-mail e senha.".into());
        }
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let exists: Option<String> = tx
            .query_row(
                "SELECT id FROM users WHERE email = ?1",
                params![email_clean],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if exists.is_some() {
            return Err("Este e-mail já está cadastrado.".into());
        }

        let uid = DateUtils::new_uuid();
        let pass_hash = hex_encode(&xor_cyclic(password.as_bytes(), b"PETMEDS_SALT_KEY"));
        tx.execute(
            "INSERT INTO users (id, name, email, password_hash, receive_email_notif)
             VALUES (?1, ?2, ?3, ?4, 1)",
            params![uid, name_clean, email_clean, pass_hash],
        )
        .map_err(|e| e.to_string())?;

        // RN44: Todo usuário possui ao menos uma família pessoal ao cadastrar
        let fam_id = DateUtils::new_uuid();
        let code = DateUtils::new_family_code();
        let primeiro_nome = split(name_clean, " ").first().copied().unwrap_or("Usuário");
        let fam_name = format!("Família de {primeiro_nome}");

        tx.execute(
            "INSERT INTO families (id, name, code, admin_id) VALUES (?1, ?2, ?3, ?4)",
            params![fam_id, fam_name, code, uid],
        )
        .map_err(|e| e.to_string())?;

        tx.execute(
            "INSERT INTO family_members (family_id, user_id, role, receive_email)
             VALUES (?1, ?2, 'admin', 1)",
            params![fam_id, uid],
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
        Self::set_pref(conn, "active_family_id", &fam_id);
        Self::set_pref(conn, "session_user_id", &uid);

        Ok(Usuario {
            id: uid,
            name: name_clean.to_string(),
            email: email_clean,
            receive_email_notif: true,
            photo: None,
        })
    }

    pub fn authenticate(conn: &Connection, email: &str, password: &str) -> Result<Usuario, String> {
        let email_clean = lower(strip(email));
        let pass_hash = hex_encode(&xor_cyclic(password.as_bytes(), b"PETMEDS_SALT_KEY"));

        let user = conn
            .query_row(
                "SELECT id, name, email, password_hash, receive_email_notif, photo
                 FROM users WHERE email = ?1",
                params![email_clean],
                |r| {
                    let db_hash: String = r.get(3)?;
                    Ok((
                        Usuario {
                            id: r.get(0)?,
                            name: r.get(1)?,
                            email: r.get(2)?,
                            receive_email_notif: r.get::<_, i32>(4)? != 0,
                            photo: r.get(5)?,
                        },
                        db_hash,
                    ))
                },
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Credenciais inválidas: usuário não encontrado.".to_string())?;

        if !constant_time_eq(user.1.as_bytes(), pass_hash.as_bytes()) {
            return Err("Credenciais inválidas: senha incorreta.".into());
        }
        Self::set_pref(conn, "session_user_id", &user.0.id);
        Ok(user.0)
    }

    pub fn reset_password_local(
        conn: &Connection,
        email: &str,
        new_pass: &str,
    ) -> Result<(), String> {
        let email_clean = lower(strip(email));
        let pass_hash = hex_encode(&xor_cyclic(new_pass.as_bytes(), b"PETMEDS_SALT_KEY"));
        let rows = conn
            .execute(
                "UPDATE users SET password_hash = ?1 WHERE email = ?2",
                params![pass_hash, email_clean],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("E-mail não encontrado no sistema.".into());
        }
        Ok(())
    }

    pub fn load_user_by_id(conn: &Connection, uid: &str) -> Option<Usuario> {
        conn.query_row(
            "SELECT id, name, email, receive_email_notif, photo FROM users WHERE id = ?1",
            params![uid],
            |r| {
                Ok(Usuario {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    email: r.get(2)?,
                    receive_email_notif: r.get::<_, i32>(3)? != 0,
                    photo: r.get(4)?,
                })
            },
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn update_profile(
        conn: &Connection,
        uid: &str,
        name: &str,
        email: &str,
        new_pass: &str,
        notif: bool,
    ) -> Result<(), String> {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE users SET name = ?1, email = ?2, receive_email_notif = ?3 WHERE id = ?4",
            params![strip(name), lower(strip(email)), i32::from(notif), uid],
        )
        .map_err(|e| e.to_string())?;

        if !strip(new_pass).is_empty() {
            let pass_hash = hex_encode(&xor_cyclic(new_pass.as_bytes(), b"PETMEDS_SALT_KEY"));
            tx.execute(
                "UPDATE users SET password_hash = ?1 WHERE id = ?2",
                params![pass_hash, uid],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn delete_account(conn: &Connection, uid: &str) -> Result<(), String> {
        conn.execute("DELETE FROM users WHERE id = ?1", params![uid])
            .map_err(|e| e.to_string())?;
        let _ = conn.execute("DELETE FROM app_prefs WHERE key = 'session_user_id'", []);
        Ok(())
    }

    // ---------------- Famílias e Membros (Módulo B) ----------------

    pub fn list_user_families(conn: &Connection, uid: &str) -> Vec<(String, String, String)> {
        let mut stmt = conn
            .prepare(
                "SELECT f.id, f.name, fm.role
                 FROM family_members fm
                 JOIN families f ON f.id = fm.family_id
                 WHERE fm.user_id = ?1
                 ORDER BY f.name",
            )
            .unwrap();
        stmt.query_map(params![uid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    }

    pub fn load_family(conn: &Connection, fam_id: &str, current_uid: &str) -> Option<Familia> {
        let (id, name, code, admin_id): (String, String, String, String) = conn
            .query_row(
                "SELECT id, name, code, admin_id FROM families WHERE id = ?1",
                params![fam_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
            .ok()??;

        let mut stmt = conn
            .prepare(
                "SELECT fm.user_id, u.name, fm.role, fm.receive_email, fm.starts_on, fm.ends_on
                 FROM family_members fm
                 JOIN users u ON u.id = fm.user_id
                 WHERE fm.family_id = ?1
                 ORDER BY u.name",
            )
            .ok()?;

        let members: Vec<MembroFamilia> = stmt
            .query_map(params![fam_id], |r| {
                Ok(MembroFamilia {
                    user_id: r.get(0)?,
                    name: r.get(1)?,
                    role: r.get(2)?,
                    receive_email: r.get::<_, i32>(3)? != 0,
                    starts_on: r.get(4)?,
                    ends_on: r.get(5)?,
                })
            })
            .ok()?
            .filter_map(Result::ok)
            .collect();

        let me = members.iter().find(|m| m.user_id == current_uid)?;
        let today = DateUtils::today_str();
        let access_state = if me.role != "cuidador_temporario" {
            "active".to_string()
        } else if me
            .starts_on
            .as_deref()
            .map(|s| today.as_str() < s)
            .unwrap_or(false)
        {
            "scheduled".to_string()
        } else if me
            .ends_on
            .as_deref()
            .map(|e| today.as_str() > e)
            .unwrap_or(false)
        {
            "expired".to_string()
        } else {
            "active".to_string()
        };

        Some(Familia {
            id,
            name,
            code,
            admin_id,
            access_state,
            starts_on: me.starts_on.clone(),
            ends_on: me.ends_on.clone(),
            members,
        })
    }

    pub fn create_family(conn: &Connection, uid: &str, name: &str) -> Result<String, String> {
        let clean = strip(name);
        if clean.is_empty() {
            return Err("Informe o nome da família.".into());
        }
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let fid = DateUtils::new_uuid();
        let code = DateUtils::new_family_code();
        tx.execute(
            "INSERT INTO families (id, name, code, admin_id) VALUES (?1, ?2, ?3, ?4)",
            params![fid, clean, code, uid],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO family_members (family_id, user_id, role, receive_email)
             VALUES (?1, ?2, 'admin', 1)",
            params![fid, uid],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Self::set_pref(conn, "active_family_id", &fid);
        Ok(fid)
    }

    pub fn join_family_by_code(conn: &Connection, uid: &str, code: &str) -> Result<String, String> {
        let code_clean = upper(strip(code));
        let fid: String = conn
            .query_row(
                "SELECT id FROM families WHERE code = ?1",
                params![code_clean],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Código de família inválido.".to_string())?;

        conn.execute(
            "INSERT OR IGNORE INTO family_members (family_id, user_id, role, receive_email)
             VALUES (?1, ?2, 'cuidador', 1)",
            params![fid, uid],
        )
        .map_err(|e| e.to_string())?;
        Self::set_pref(conn, "active_family_id", &fid);
        Ok(fid)
    }

    pub fn invite_member(
        conn: &Connection,
        fam_id: &str,
        email: &str,
        role: &str,
        starts_on: Option<String>,
        ends_on: Option<String>,
    ) -> Result<(), String> {
        if let (Some(s), Some(e)) = (&starts_on, &ends_on) {
            if !s.is_empty() && !e.is_empty() && e < s {
                return Err("A data final deve ser igual ou posterior à data de início.".into());
            }
        }
        let target_uid: String = conn
            .query_row(
                "SELECT id FROM users WHERE email = ?1",
                params![lower(strip(email))],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Usuario nao encontrado".to_string())?;

        let s_val = if role == "cuidador_temporario" {
            starts_on.filter(|x| !x.is_empty())
        } else {
            None
        };
        let e_val = if role == "cuidador_temporario" {
            ends_on.filter(|x| !x.is_empty())
        } else {
            None
        };

        conn.execute(
            "INSERT INTO family_members (family_id, user_id, role, receive_email, starts_on, ends_on)
             VALUES (?1, ?2, ?3, 1, ?4, ?5)
             ON CONFLICT(family_id, user_id) DO UPDATE
             SET role = excluded.role, starts_on = excluded.starts_on, ends_on = excluded.ends_on",
            params![fam_id, target_uid, role, s_val, e_val],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_member_role_or_period(
        conn: &Connection,
        fam_id: &str,
        user_id: &str,
        role: &str,
        starts_on: Option<String>,
        ends_on: Option<String>,
    ) -> Result<(), String> {
        if let (Some(s), Some(e)) = (&starts_on, &ends_on) {
            if !s.is_empty() && !e.is_empty() && e < s {
                return Err("A data final deve ser igual ou posterior à data de início.".into());
            }
        }
        conn.execute(
            "UPDATE family_members SET role = ?1, starts_on = ?2, ends_on = ?3
             WHERE family_id = ?4 AND user_id = ?5",
            params![
                role,
                starts_on.filter(|x| !x.is_empty()),
                ends_on.filter(|x| !x.is_empty()),
                fam_id,
                user_id
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn remove_member(conn: &Connection, fam_id: &str, user_id: &str) -> Result<(), String> {
        conn.execute(
            "DELETE FROM family_members WHERE family_id = ?1 AND user_id = ?2",
            params![fam_id, user_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn rename_family(conn: &Connection, fam_id: &str, new_name: &str) -> Result<(), String> {
        conn.execute(
            "UPDATE families SET name = ?1 WHERE id = ?2",
            params![strip(new_name), fam_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_family(
        conn: &Connection,
        fam_id: &str,
        user: &Usuario,
    ) -> Result<String, String> {
        let fams = Self::list_user_families(conn, &user.id);
        let was_last = len(&fams) <= 1;
        conn.execute("DELETE FROM families WHERE id = ?1", params![fam_id])
            .map_err(|e| e.to_string())?;
        if was_last {
            let first = split(&user.name, " ").first().copied().unwrap_or("Usuário");
            Self::create_family(conn, &user.id, &format!("Família de {first}"))
        } else {
            let remaining = Self::list_user_families(conn, &user.id);
            let next_id = remaining.first().map(|f| f.0.clone()).unwrap_or_default();
            Self::set_pref(conn, "active_family_id", &next_id);
            Ok(next_id)
        }
    }

    // ---------------- Pets e Pesos (Módulo C) ----------------

    pub fn load_pets(conn: &Connection, fam_id: &str) -> Vec<Pet> {
        let mut stmt = conn
            .prepare(
                "SELECT id, family_id, name, species, birth_date, status, donated_at, deceased_at, photo
                 FROM pets WHERE family_id = ?1 ORDER BY name",
            )
            .unwrap();

        let mut pets: Vec<Pet> = stmt
            .query_map(params![fam_id], |r| {
                let base = EntidadeBase::new(r.get(0)?, r.get(1)?);
                Ok(Pet::new(
                    base,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    Vec::new(),
                ))
            })
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        let mut w_stmt = conn
            .prepare(
                "SELECT id, weight, recorded_at FROM weight_records
                 WHERE pet_id = ?1 ORDER BY recorded_at ASC",
            )
            .unwrap();

        for pet in pets.iter_mut() {
            if let Ok(rows) = w_stmt.query_map(params![pet.id], |r| {
                Ok(RegistroPeso {
                    id: r.get(0)?,
                    weight: r.get(1)?,
                    date: r.get(2)?,
                })
            }) {
                pet.weight_history = rows.filter_map(Result::ok).collect();
            }
        }
        pets
    }

    pub fn save_pet(
        conn: &Connection,
        id: Option<&str>,
        fam_id: &str,
        name: &str,
        species: &str,
        birth: &str,
        status: &str,
        donated_at: Option<String>,
        deceased_at: Option<String>,
    ) -> Result<(), String> {
        let clean_name = strip(name);
        if clean_name.is_empty() || strip(birth).is_empty() {
            return Err("Nome e data de nascimento são obrigatórios.".into());
        }
        let today = DateUtils::today_str();
        let don = if status == "donated" {
            Some(
                donated_at
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| today.clone()),
            )
        } else {
            donated_at.filter(|s| !s.is_empty())
        };
        let dec = if status == "deceased" {
            Some(deceased_at.filter(|s| !s.is_empty()).unwrap_or(today))
        } else {
            deceased_at.filter(|s| !s.is_empty())
        };

        if let Some(pid) = id.filter(|s| !s.is_empty()) {
            conn.execute(
                "UPDATE pets SET name = ?1, species = ?2, birth_date = ?3, status = ?4,
                 donated_at = ?5, deceased_at = ?6 WHERE id = ?7",
                params![clean_name, species, birth, status, don, dec, pid],
            )
            .map_err(|e| e.to_string())?;
        } else {
            let pid = DateUtils::new_uuid();
            conn.execute(
                "INSERT INTO pets (id, family_id, name, species, birth_date, status, donated_at, deceased_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![pid, fam_id, clean_name, species, birth, status, don, dec],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn delete_pet(conn: &Connection, pet_id: &str) -> Result<(), String> {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM doses WHERE pet_id = ?1", params![pet_id])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM treatments WHERE pet_id = ?1", params![pet_id])
            .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM weight_records WHERE pet_id = ?1",
            params![pet_id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM pets WHERE id = ?1", params![pet_id])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn save_weight(
        conn: &Connection,
        weight_id: Option<&str>,
        pet_id: &str,
        weight: f64,
        date: &str,
    ) -> Result<(), String> {
        if weight <= 0.0 || strip(date).is_empty() {
            return Err("Informe um peso válido maior que zero e a data.".into());
        }
        if let Some(wid) = weight_id.filter(|s| !s.is_empty()) {
            conn.execute(
                "UPDATE weight_records SET weight = ?1, recorded_at = ?2 WHERE id = ?3",
                params![weight, date, wid],
            )
            .map_err(|e| e.to_string())?;
        } else {
            let wid = DateUtils::new_uuid();
            conn.execute(
                "INSERT INTO weight_records (id, pet_id, weight, recorded_at) VALUES (?1, ?2, ?3, ?4)",
                params![wid, pet_id, weight, date],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ---------------- Medicamentos e Estoque (Módulo D) ----------------

    pub fn load_medications(conn: &Connection, fam_id: &str) -> Vec<Medicamento> {
        let mut stmt = conn
            .prepare(
                "SELECT id, family_id, name, lab, quantity, package_quantity, single_dose, unit, bought_at, expires_at
                 FROM medications WHERE family_id = ?1 ORDER BY name",
            )
            .unwrap();

        let mut meds: Vec<Medicamento> = stmt
            .query_map(params![fam_id], |r| {
                let base = EntidadeBase::new(r.get(0)?, r.get(1)?);
                Ok(Medicamento::new(
                    base,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get::<_, i32>(6)? != 0,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                    Vec::new(),
                ))
            })
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        let mut h_stmt = conn
            .prepare(
                "SELECT move_type, purchased_at, expires_at, quantity_added, recorded_at
                 FROM stock_history WHERE medication_id = ?1 ORDER BY COALESCE(purchased_at, ''), recorded_at",
            )
            .unwrap();

        for med in meds.iter_mut() {
            if let Ok(rows) = h_stmt.query_map(params![med.id], |r| {
                Ok(MovimentacaoEstoque {
                    move_type: r.get(0)?,
                    purchased_at: r.get(1)?,
                    expires_at: r.get(2)?,
                    quantity_added: r.get(3)?,
                    recorded_at: r.get(4)?,
                })
            }) {
                med.stock_history = rows.filter_map(Result::ok).collect();
            }
        }
        meds
    }

    pub fn save_medication(
        conn: &Connection,
        id: Option<&str>,
        fam_id: &str,
        name: &str,
        lab: &str,
        qty: f64,
        pkg_qty: Option<f64>,
        single_dose: bool,
        unit: &str,
        buy_date: Option<String>,
        exp_date: &str,
    ) -> Result<(), String> {
        if strip(name).is_empty()
            || strip(lab).is_empty()
            || strip(exp_date).is_empty()
            || qty < 0.0
        {
            return Err("Preencha nome, laboratório, validade e quantidade não negativa.".into());
        }
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let buy_clean = buy_date.filter(|s| !s.is_empty());

        if let Some(mid) = id.filter(|s| !s.is_empty()) {
            let old: (f64, Option<String>, String) = tx
                .query_row(
                    "SELECT quantity, bought_at, expires_at FROM medications WHERE id = ?1",
                    params![mid],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .map_err(|e| e.to_string())?;

            let changed = (qty - old.0).abs() > 1e-6 || buy_clean != old.1 || exp_date != old.2;
            tx.execute(
                "UPDATE medications SET name = ?1, lab = ?2, quantity = ?3, package_quantity = ?4,
                 single_dose = ?5, unit = ?6, bought_at = ?7, expires_at = ?8 WHERE id = ?9",
                params![
                    strip(name),
                    strip(lab),
                    qty,
                    pkg_qty,
                    i32::from(single_dose),
                    unit,
                    buy_clean,
                    exp_date,
                    mid
                ],
            )
            .map_err(|e| e.to_string())?;

            if changed {
                tx.execute(
                    "INSERT INTO stock_history (id, medication_id, move_type, purchased_at, expires_at, quantity_added, recorded_at)
                     VALUES (?1, ?2, 'adjustment', ?3, ?4, ?5, ?6)",
                    params![
                        DateUtils::new_uuid(),
                        mid,
                        buy_clean,
                        exp_date,
                        qty - old.0,
                        DateUtils::now_iso_str()
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
        } else {
            let mid = DateUtils::new_uuid();
            tx.execute(
                "INSERT INTO medications (id, family_id, name, lab, quantity, package_quantity, single_dose, unit, bought_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    mid,
                    fam_id,
                    strip(name),
                    strip(lab),
                    qty,
                    pkg_qty,
                    i32::from(single_dose),
                    unit,
                    buy_clean,
                    exp_date
                ],
            )
            .map_err(|e| e.to_string())?;

            tx.execute(
                "INSERT INTO stock_history (id, medication_id, move_type, purchased_at, expires_at, quantity_added, recorded_at)
                 VALUES (?1, ?2, 'initial', ?3, ?4, ?5, ?6)",
                params![
                    DateUtils::new_uuid(),
                    mid,
                    buy_clean,
                    exp_date,
                    qty,
                    DateUtils::now_iso_str()
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn restock_medication(
        conn: &Connection,
        med_id: &str,
        qty_added: f64,
        purchased_at: &str,
        expires_at: &str,
    ) -> Result<(), String> {
        if qty_added <= 0.0 || strip(purchased_at).is_empty() || strip(expires_at).is_empty() {
            return Err("Informe quantidade maior que zero, data da compra e validade.".into());
        }
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE medications SET quantity = quantity + ?1, bought_at = ?2, expires_at = ?3 WHERE id = ?4",
            params![qty_added, purchased_at, expires_at, med_id],
        )
        .map_err(|e| e.to_string())?;

        tx.execute(
            "INSERT INTO stock_history (id, medication_id, move_type, purchased_at, expires_at, quantity_added, recorded_at)
             VALUES (?1, ?2, 'restock', ?3, ?4, ?5, ?6)",
            params![
                DateUtils::new_uuid(),
                med_id,
                purchased_at,
                expires_at,
                qty_added,
                DateUtils::now_iso_str()
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn toggle_single_dose(
        conn: &Connection,
        med_id: &str,
        enabled: bool,
    ) -> Result<(), String> {
        conn.execute(
            "UPDATE medications SET single_dose = ?1 WHERE id = ?2",
            params![i32::from(enabled), med_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ---------------- Tratamentos e Doses (Módulos E & F) ----------------

    pub fn load_treatments(conn: &Connection, fam_id: &str) -> Vec<Tratamento> {
        let mut stmt = conn
            .prepare(
                "SELECT id, family_id, COALESCE(code, '--'), pet_id, medication_id, dosage,
                        dosage_quantity, dosage_unit, schedule_mode, tolerance_minutes,
                        start_date, interval_value, interval_unit, duration_type, end_date,
                        paused_at, finished_at, created_at
                 FROM treatments WHERE family_id = ?1 ORDER BY created_at DESC",
            )
            .unwrap();

        let mut list: Vec<Tratamento> = stmt
            .query_map(params![fam_id], |r| {
                let base = EntidadeBase::new(r.get(0)?, r.get(1)?);
                Ok(Tratamento::new(
                    base,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                    r.get(10)?,
                    r.get(11)?,
                    r.get(12)?,
                    r.get(13)?,
                    r.get(14)?,
                    r.get(15)?,
                    r.get(16)?,
                    r.get(17)?,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                ))
            })
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        let mut t_stmt = conn
            .prepare("SELECT time_str, enabled FROM treatment_times WHERE treatment_id = ?1 ORDER BY time_str")
            .unwrap();
        let mut p_stmt = conn
            .prepare("SELECT id, paused_at, resumed_at FROM pause_history WHERE treatment_id = ?1 ORDER BY paused_at")
            .unwrap();

        for tr in list.iter_mut() {
            if let Ok(rows) = t_stmt.query_map(params![tr.id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i32>(1)? != 0))
            }) {
                for (t_str, en) in rows.filter_map(Result::ok) {
                    tr.times.append_item(t_str.clone());
                    if !en {
                        tr.disabled_times.append_item(t_str);
                    }
                }
            }
            if let Ok(rows) = p_stmt.query_map(params![tr.id], |r| {
                Ok(PeriodoPausa {
                    id: r.get(0)?,
                    paused_at: r.get(1)?,
                    resumed_at: r.get(2)?,
                })
            }) {
                tr.pause_history = rows.filter_map(Result::ok).collect();
            }
        }
        list
    }

    pub fn load_doses(conn: &Connection, fam_id: &str) -> Vec<Dose> {
        let mut stmt = conn
            .prepare(
                "SELECT id, family_id, treatment_id, pet_id, medication_id, scheduled_at,
                        actual_at, administered_by, status, notified
                 FROM doses WHERE family_id = ?1 ORDER BY scheduled_at ASC",
            )
            .unwrap();

        stmt.query_map(params![fam_id], |r| {
            let base = EntidadeBase::new(r.get(0)?, r.get(1)?);
            Ok(Dose::new(
                base,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get::<_, i32>(9)? != 0,
            ))
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }
}
