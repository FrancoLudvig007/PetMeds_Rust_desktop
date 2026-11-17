// src/db.rs
use rusqlite::{Connection, Result};

pub struct Database;

impl Database {
    pub fn connect() -> Result<Connection> {
        let conn = Connection::open("petmeds_local.db")?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )?;
        Self::migrate(&conn)?;
        Ok(conn)
    }

    fn migrate(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS app_prefs (
                key TEXT PRIMARY KEY,
                val TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                receive_email_notif INTEGER NOT NULL DEFAULT 1,
                photo TEXT
            );

            CREATE TABLE IF NOT EXISTS families (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                code TEXT NOT NULL UNIQUE,
                admin_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS family_members (
                family_id TEXT NOT NULL REFERENCES families(id) ON DELETE CASCADE,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                role TEXT NOT NULL CHECK (role IN ('admin', 'cuidador', 'visualizador', 'cuidador_temporario')),
                receive_email INTEGER NOT NULL DEFAULT 1,
                starts_on TEXT,
                ends_on TEXT,
                PRIMARY KEY (family_id, user_id)
            );

            CREATE TABLE IF NOT EXISTS pets (
                id TEXT PRIMARY KEY,
                family_id TEXT NOT NULL REFERENCES families(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                species TEXT NOT NULL,
                birth_date TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'donated', 'lost', 'deceased')),
                donated_at TEXT,
                deceased_at TEXT,
                photo TEXT
            );

            CREATE TABLE IF NOT EXISTS weight_records (
                id TEXT PRIMARY KEY,
                pet_id TEXT NOT NULL REFERENCES pets(id) ON DELETE CASCADE,
                weight REAL NOT NULL CHECK (weight > 0),
                recorded_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS medications (
                id TEXT PRIMARY KEY,
                family_id TEXT NOT NULL REFERENCES families(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                lab TEXT NOT NULL,
                quantity REAL NOT NULL DEFAULT 0 CHECK (quantity >= 0),
                package_quantity REAL,
                single_dose INTEGER NOT NULL DEFAULT 0,
                unit TEXT NOT NULL,
                bought_at TEXT,
                expires_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stock_history (
                id TEXT PRIMARY KEY,
                medication_id TEXT NOT NULL REFERENCES medications(id) ON DELETE CASCADE,
                move_type TEXT NOT NULL CHECK (move_type IN ('initial', 'restock', 'adjustment')),
                purchased_at TEXT,
                expires_at TEXT NOT NULL,
                quantity_added REAL NOT NULL,
                recorded_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS treatments (
                id TEXT PRIMARY KEY,
                code TEXT UNIQUE,
                family_id TEXT NOT NULL REFERENCES families(id) ON DELETE CASCADE,
                pet_id TEXT NOT NULL REFERENCES pets(id) ON DELETE CASCADE,
                medication_id TEXT NOT NULL REFERENCES medications(id) ON DELETE CASCADE,
                dosage TEXT NOT NULL,
                dosage_quantity REAL NOT NULL CHECK (dosage_quantity > 0),
                dosage_unit TEXT NOT NULL,
                schedule_mode TEXT NOT NULL DEFAULT 'exact' CHECK (schedule_mode IN ('exact', 'window', 'all_day')),
                tolerance_minutes INTEGER NOT NULL DEFAULT 0,
                start_date TEXT NOT NULL,
                interval_value INTEGER NOT NULL DEFAULT 1 CHECK (interval_value >= 1),
                interval_unit TEXT NOT NULL DEFAULT 'day' CHECK (interval_unit IN ('day', 'week', 'month', 'year')),
                duration_type TEXT NOT NULL DEFAULT 'indefinite' CHECK (duration_type IN ('indefinite', 'until_date')),
                end_date TEXT,
                paused_at TEXT,
                finished_at TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS treatment_times (
                treatment_id TEXT NOT NULL REFERENCES treatments(id) ON DELETE CASCADE,
                time_str TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (treatment_id, time_str)
            );

            CREATE TABLE IF NOT EXISTS pause_history (
                id TEXT PRIMARY KEY,
                treatment_id TEXT NOT NULL REFERENCES treatments(id) ON DELETE CASCADE,
                paused_at TEXT NOT NULL,
                resumed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS doses (
                id TEXT PRIMARY KEY,
                family_id TEXT NOT NULL REFERENCES families(id) ON DELETE CASCADE,
                treatment_id TEXT REFERENCES treatments(id) ON DELETE CASCADE,
                pet_id TEXT NOT NULL REFERENCES pets(id) ON DELETE CASCADE,
                medication_id TEXT NOT NULL REFERENCES medications(id) ON DELETE CASCADE,
                scheduled_at TEXT NOT NULL,
                actual_at TEXT,
                administered_by TEXT REFERENCES users(id) ON DELETE SET NULL,
                status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'late', 'done', 'skipped')),
                notified INTEGER NOT NULL DEFAULT 0,
                UNIQUE (treatment_id, scheduled_at)
            );

            -- Trigger RN13: Impede reativação de pet falecido
            CREATE TRIGGER IF NOT EXISTS trg_prevent_deceased_reactivation
            BEFORE UPDATE OF status ON pets
            WHEN OLD.status = 'deceased' AND NEW.status <> 'deceased'
            BEGIN
                SELECT RAISE(ABORT, 'Pets falecidos nao podem ser reativados');
            END;

            -- Trigger RN20: Baixa e devolução automática de estoque ao alterar dose
            CREATE TRIGGER IF NOT EXISTS trg_doses_sync_stock_insert
            AFTER INSERT ON doses
            WHEN NEW.status = 'done'
            BEGIN
                UPDATE medications
                SET quantity = MAX(quantity - 1.0, 0.0)
                WHERE id = NEW.medication_id;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_doses_sync_stock_update
            AFTER UPDATE OF status ON doses
            BEGIN
                UPDATE medications
                SET quantity = CASE
                    WHEN NEW.status = 'done' AND OLD.status <> 'done' THEN MAX(quantity - 1.0, 0.0)
                    WHEN OLD.status = 'done' AND NEW.status <> 'done' THEN quantity + 1.0
                    ELSE quantity
                END
                WHERE id = NEW.medication_id;
            END;

            -- Trigger RN33: Código sequencial de 8 dígitos para tratamentos
            CREATE TRIGGER IF NOT EXISTS trg_treatments_assign_code
            AFTER INSERT ON treatments
            WHEN NEW.code IS NULL OR NEW.code = ''
            BEGIN
                UPDATE treatments
                SET code = printf('%08d', NEW.rowid)
                WHERE rowid = NEW.rowid;
            END;
            ",
        )
    }
}
