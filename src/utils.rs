use crate::pyrsutils::prelude::*;

pub fn gerar_uuid() -> String {
    let mut bytes = [0u8; 16];
    for i in range(16) {
        bytes[i as usize] = randint(0, 255) as u8;
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

pub fn gerar_codigo_familia() -> String {
    let alfabeto: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
    let mut codigo = String::with_capacity(6);
    for _ in range(6) {
        if let Some(&c) = choice(&alfabeto) {
            codigo.push(c);
        }
    }
    upper(&codigo)
}

pub fn hoje_iso() -> String {
    let agora = gmtime(time().floor() as i64 - 3 * 3600);
    strftime("%Y-%m-%d", &agora)
}

pub fn agora_iso_minuto() -> String {
    let agora = gmtime(time().floor() as i64 - 3 * 3600);
    strftime("%Y-%m-%dT%H:%M", &agora)
}

pub fn agora_iso_completo() -> String {
    let agora = gmtime(time().floor() as i64 - 3 * 3600);
    strftime("%Y-%m-%dT%H:%M:%S", &agora)
}

pub fn formatar_data_br(data_iso: &str) -> String {
    let limpo = strip(data_iso);
    if len(limpo) < 10 {
        return limpo.to_string();
    }
    let partes = split(&limpo[0..10], "-");
    if len(&partes) == 3 {
        format!("{}/{}/{}", partes[2], partes[1], partes[0])
    } else {
        limpo.to_string()
    }
}

pub fn formatar_data_hora_br(dt_iso: &str) -> String {
    let limpo = strip(dt_iso);
    if len(limpo) >= 16 {
        format!("{} às {}", formatar_data_br(&limpo[0..10]), &limpo[11..16])
    } else {
        formatar_data_br(limpo)
    }
}

pub fn data_iso_para_epoch(data_iso: &str) -> Option<i64> {
    let limpo = strip(data_iso);
    if len(limpo) < 10 {
        return None;
    }
    let partes = split(&limpo[0..10], "-");
    if len(&partes) != 3 {
        return None;
    }
    let year: i64 = partes[0].parse().ok()?;
    let month: u32 = partes[1].parse().ok()?;
    let day: u32 = partes[2].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(mktime_utc(&StructTime {
        year,
        month,
        day,
        hour: 12,
        minute: 0,
        second: 0,
        weekday: 0,
        yearday: 1,
    }))
}

pub fn datetime_iso_para_epoch(dt_iso: &str) -> Option<i64> {
    let limpo = strip(dt_iso);
    if len(limpo) < 16 {
        return None;
    }
    let d = split(&limpo[0..10], "-");
    let h = split(&limpo[11..16], ":");
    if len(&d) != 3 || len(&h) != 2 {
        return None;
    }
    Some(mktime_utc(&StructTime {
        year: d[0].parse().ok()?,
        month: d[1].parse().ok()?,
        day: d[2].parse().ok()?,
        hour: h[0].parse().ok()?,
        minute: h[1].parse().ok()?,
        second: 0,
        weekday: 0,
        yearday: 1,
    }))
}

pub fn somar_dias_iso(data_iso: &str, dias: i64) -> String {
    if let Some(ep) = data_iso_para_epoch(data_iso) {
        strftime("%Y-%m-%d", &gmtime(ep + dias * 86_400))
    } else {
        data_iso.to_string()
    }
}

pub fn ultimo_dia_do_mes(ano: i64, mes: u32) -> u32 {
    match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (ano % 4 == 0 && ano % 100 != 0) || (ano % 400 == 0) => 29,
        2 => 28,
        _ => 30,
    }
}

pub fn adicionar_intervalo_tratamento(
    data_inicio: &str,
    interval_unit: &str,
    interval_value: i64,
    ocorrencia: i64,
) -> Option<String> {
    let st = gmtime(data_iso_para_epoch(data_inicio)?);
    let passos = ocorrencia.saturating_mul(interval_value);

    match interval_unit {
        "day" => Some(somar_dias_iso(data_inicio, passos)),
        "week" => Some(somar_dias_iso(data_inicio, passos * 7)),
        "month" => {
            let total_meses = (st.year * 12 + (st.month as i64 - 1)) + passos;
            let novo_ano = total_meses.div_euclid(12);
            let novo_mes = (total_meses.rem_euclid(12) + 1) as u32;
            let novo_dia = st.day.min(ultimo_dia_do_mes(novo_ano, novo_mes));
            Some(format!("{novo_ano:04}-{novo_mes:02}-{novo_dia:02}"))
        }
        "year" => {
            let novo_ano = st.year + passos;
            let novo_dia = st.day.min(ultimo_dia_do_mes(novo_ano, st.month));
            Some(format!("{novo_ano:04}-{:02}-{novo_dia:02}", st.month))
        }
        _ => Some(somar_dias_iso(data_inicio, passos)),
    }
}

pub fn hash_senha(senha: &str) -> String {
    let chave = b"PetMeds_Desktop_Salt_2026_PyRsUtils";
    hex_encode(&xor_cyclic(senha.as_bytes(), chave))
}

pub fn verificar_senha(senha_digitada: &str, hash_salvo: &str) -> bool {
    constant_time_eq(hash_senha(senha_digitada).as_bytes(), hash_salvo.as_bytes())
}
