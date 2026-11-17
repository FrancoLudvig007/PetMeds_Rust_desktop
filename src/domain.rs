use crate::class;
use crate::pyrsutils::prelude::*;
use crate::utils::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Polimorfismo via Traits (Controle de Acesso RN01-RN05 e Cartões de UI)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelSeveridade {
    Sucesso,
    Aviso,
    Perigo,
    Neutro,
}

pub trait ResumoCartao {
    fn icone_cartao(&self) -> &'static str;
    fn titulo_cartao(&self) -> String;
    fn subtitulo_cartao(&self) -> String;
    fn selo_status(&self, hoje: &str) -> Option<(String, NivelSeveridade)>;
}

pub trait PoliticaPapel {
    fn rotulo_papel(&self) -> &'static str;
    fn pode_gerenciar_familia(&self) -> bool;
    fn pode_editar_cadastros(&self, hoje: &str) -> bool;
    fn pode_registrar_doses(&self, hoje: &str) -> bool;
    fn estado_acesso(&self, hoje: &str) -> &'static str;
}

// ============================================================================
// Estruturas de Histórico Auxiliares
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistroPeso {
    pub id: String,
    pub pet_id: String,
    pub weight: f64,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovimentacaoEstoque {
    pub tipo: String,
    pub purchased_at: Option<String>,
    pub expires_at: String,
    pub quantity_added: f64,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeriodoPausa {
    pub paused_at: String,
    pub resumed_at: Option<String>,
}

#[derive(Serialize)]
pub struct BackupFamiliaJson {
    pub exported_at: String,
    pub family_name: String,
    pub family_code: String,
    pub pets_count: usize,
    pub medications_count: usize,
    pub treatments_count: usize,
    pub doses_count: usize,
    pub pets: Vec<String>,
    pub medications: Vec<String>,
    pub treatments: Vec<String>,
}

// ============================================================================
// Superclasses Base e Herança com `class!` (Sem `///` dentro do bloco macro!)
// ============================================================================

class! {
    struct IdentidadeBase {
        id: String,
        nome: String,
    }
    impl {
        fn inicial(&self) -> String {
            self.nome
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string()
        }
    }
}

class! {
    struct EntidadeFamiliar {
        id: String,
        family_id: String,
    }
    impl {
        fn pertence_a_familia(&self, fid: &str) -> bool {
            self.family_id == fid
        }
    }
}

// Herança: Usuario herda de IdentidadeBase (`id` e `nome` via Deref/DerefMut)
class! {
    struct Usuario : IdentidadeBase {
        email: String,
        senha_hash: String,
        receive_email_notif: bool,
        photo: Option<String>,
    }
    impl {
        // Método do diagrama UML: +atualizarPerfil(nome) (RF02)
        fn atualizar_perfil(&mut self, novo_nome: String, novo_email: String, nova_foto: Option<String>) {
            self.nome = strip(&novo_nome).to_string();
            self.email = lower(strip(&novo_email));
            self.photo = nova_foto;
        }

        fn primeiro_nome(&self) -> &str {
            split(&self.nome, " ").first().copied().unwrap_or("Usuário")
        }
    }
}

class! {
    struct MembroFamilia : EntidadeFamiliar {
        user_id: String,
        user_name: String,
        papel: String,
        receive_email: bool,
        inicio_acesso: Option<String>,
        fim_acesso: Option<String>,
    }
    impl {
        // Método do diagrama UML: +verificarAcesso(data) (RF04, RN05)
        fn verificar_acesso(&self, data_atual: &str) -> &'static str {
            if self.papel != "cuidador_temporario" {
                return "active";
            }
            if let Some(ref ini) = self.inicio_acesso {
                if !ini.is_empty() && data_atual < ini.as_str() {
                    return "scheduled";
                }
            }
            if let Some(ref fim) = self.fim_acesso {
                if !fim.is_empty() && data_atual > fim.as_str() {
                    return "expired";
                }
            }
            "active"
        }
    }
}

impl PoliticaPapel for MembroFamilia {
    fn rotulo_papel(&self) -> &'static str {
        match self.papel.as_str() {
            "admin" => "Admin",
            "cuidador" => "Cuidador",
            "cuidador_temporario" => "Cuidador temporário",
            "visualizador" => "Visualizador",
            _ => "Membro",
        }
    }

    fn pode_gerenciar_familia(&self) -> bool {
        self.papel == "admin"
    }

    fn pode_editar_cadastros(&self, hoje: &str) -> bool {
        matches!(self.papel.as_str(), "admin" | "cuidador")
            && self.verificar_acesso(hoje) == "active"
    }

    fn pode_registrar_doses(&self, hoje: &str) -> bool {
        matches!(
            self.papel.as_str(),
            "admin" | "cuidador" | "cuidador_temporario"
        ) && self.verificar_acesso(hoje) == "active"
    }

    fn estado_acesso(&self, hoje: &str) -> &'static str {
        self.verificar_acesso(hoje)
    }
}

// Herança: Familia herda de IdentidadeBase (`id` e `nome`)
class! {
    struct Familia : IdentidadeBase {
        codigo: String,
        admin_id: String,
        membros: Vec<MembroFamilia>,
    }
    impl {
        // Método do diagrama UML: +adicionarMembro(usuario, papel) (RF04, RN01)
        fn adicionar_membro(&mut self, usuario: &Usuario, papel: &str, inicio: Option<String>, fim: Option<String>) {
            if let Some(existente) = self.membros.iter_mut().find(|m| m.user_id == usuario.id) {
                existente.papel = papel.to_string();
                existente.inicio_acesso = inicio;
                existente.fim_acesso = fim;
            } else {
                self.membros.append_item(MembroFamilia::new(
                    EntidadeFamiliar::new(gerar_uuid(), self.id.clone()),
                    usuario.id.clone(),
                    usuario.nome.clone(),
                    papel.to_string(),
                    true,
                    inicio,
                    fim,
                ));
            }
        }
    }
}

// Herança: Pet herda de EntidadeFamiliar (`id` e `family_id`)
class! {
    struct Pet : EntidadeFamiliar {
        nome: String,
        especie: String,
        data_nascimento: String,
        status: String,
        donated_at: Option<String>,
        deceased_at: Option<String>,
        photo: Option<String>,
        weight_history: Vec<RegistroPeso>,
    }
    impl {
        // Método do diagrama UML: +atualizarStatus(status) (RF05, RN06)
        fn atualizar_status(&mut self, novo_status: &str, data_evento: Option<String>) -> Result<(), String> {
            if self.status == "deceased" && novo_status != "deceased" {
                return Err("RN06: Pets falecidos não podem ser reativados.".to_string());
            }
            self.status = novo_status.to_string();
            let hoje = hoje_iso();
            if novo_status == "donated" {
                self.donated_at = Some(data_evento.unwrap_or(hoje));
            } else if novo_status == "deceased" {
                self.deceased_at = Some(data_evento.unwrap_or(hoje));
            }
            Ok(())
        }

        fn peso_atual(&self) -> Option<&RegistroPeso> {
            self.weight_history.last()
        }

        fn rotulo_status(&self) -> &'static str {
            match self.status.as_str() {
                "active" => "Ativo",
                "donated" => "Doado",
                "lost" => "Perdido",
                "deceased" => "Falecido",
                _ => "Ativo",
            }
        }
    }
}

impl ResumoCartao for Pet {
    fn icone_cartao(&self) -> &'static str {
        match self.especie.as_str() {
            "Gato" => "🐱",
            "Cão" => "🐶",
            _ => "🐾",
        }
    }

    fn titulo_cartao(&self) -> String {
        self.nome.clone()
    }

    fn subtitulo_cartao(&self) -> String {
        format!(
            "Espécie: {} • 📅 Nasc.: {}",
            self.especie,
            formatar_data_br(&self.data_nascimento)
        )
    }

    fn selo_status(&self, _hoje: &str) -> Option<(String, NivelSeveridade)> {
        let sev = match self.status.as_str() {
            "active" => NivelSeveridade::Sucesso,
            "deceased" => NivelSeveridade::Perigo,
            _ => NivelSeveridade::Aviso,
        };
        Some((self.rotulo_status().to_string(), sev))
    }
}

// Herança: Medicamento herda de EntidadeFamiliar (`id` e `family_id`)
class! {
    struct Medicamento : EntidadeFamiliar {
        nome: String,
        lab: String,
        quantidade: f64,
        package_quantity: Option<f64>,
        single_dose: bool,
        unit: String,
        bought_at: Option<String>,
        data_validade: String,
        stock_history: Vec<MovimentacaoEstoque>,
    }
    impl {
        // Método do diagrama UML: +adicionarEstoque(quantidade) (RF08, RN07)
        fn adicionar_estoque(&mut self, qtd: f64, data_compra: String, nova_validade: String) {
            if qtd > 0.0 {
                self.quantidade += qtd;
                self.bought_at = Some(data_compra.clone());
                self.data_validade = nova_validade.clone();
                self.stock_history.append_item(MovimentacaoEstoque {
                    tipo: "restock".to_string(),
                    purchased_at: Some(data_compra),
                    expires_at: nova_validade,
                    quantity_added: qtd,
                    recorded_at: agora_iso_completo(),
                });
            }
        }

        // Método do diagrama UML: +baixarEstoque(quantidade) (RN08)
        fn baixar_estoque(&mut self, qtd: f64) {
            self.quantidade = (self.quantidade - qtd).max(0.0);
        }

        fn categoria_estoque(&self, hoje: &str) -> &'static str {
            if !self.data_validade.is_empty() && self.data_validade.as_str() < hoje {
                "expired"
            } else if self.quantidade > 0.0 {
                "available"
            } else {
                "empty"
            }
        }

        fn vence_em_breve(&self, hoje: &str) -> bool {
            let limite = somar_dias_iso(hoje, 30);
            self.data_validade.as_str() >= hoje && self.data_validade.as_str() <= limite.as_str()
        }
    }
}

impl ResumoCartao for Medicamento {
    fn icone_cartao(&self) -> &'static str {
        "💊"
    }

    fn titulo_cartao(&self) -> String {
        self.nome.clone()
    }

    fn subtitulo_cartao(&self) -> String {
        let emb = self
            .package_quantity
            .map(|p| format!(" | Embalagem: {p} {}", self.unit))
            .unwrap_or_default();
        format!(
            "Lab: {} | Disponível: {} {}{}",
            self.lab, self.quantidade, self.unit, emb
        )
    }

    fn selo_status(&self, hoje: &str) -> Option<(String, NivelSeveridade)> {
        if self.categoria_estoque(hoje) == "expired" {
            Some(("FORA DA VALIDADE".to_string(), NivelSeveridade::Perigo))
        } else if self.vence_em_breve(hoje) {
            Some(("VENCE EM BREVE".to_string(), NivelSeveridade::Aviso))
        } else {
            None
        }
    }
}

// Herança: Tratamento herda de EntidadeFamiliar (`id` e `family_id`)
class! {
    struct Tratamento : EntidadeFamiliar {
        code: String,
        pet_id: String,
        medication_id: String,
        dosagem: String,
        dosage_quantity: f64,
        dosage_unit: String,
        times: Vec<String>,
        disabled_times: Vec<String>,
        schedule_mode: String,
        tolerance_minutes: i32,
        data_inicio: String,
        interval_value: i64,
        interval_unit: String,
        interval_days: i64,
        duration_type: String,
        data_fim: Option<String>,
        paused_at: Option<String>,
        finished_at: Option<String>,
        pause_history: Vec<PeriodoPausa>,
    }
    impl {
        fn esta_concluido(&self, hoje: &str) -> bool {
            if self.finished_at.is_some() {
                return true;
            }
            if let Some(ref fim) = self.data_fim {
                if !fim.is_empty() && fim.as_str() < hoje {
                    return true;
                }
            }
            false
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

        // Método do diagrama UML: +pausar() (RF10)
        fn pausar(&mut self) {
            let agora = agora_iso_completo();
            self.paused_at = Some(agora.clone());
            self.pause_history.append_item(PeriodoPausa {
                paused_at: agora,
                resumed_at: None,
            });
        }

        fn retomar(&mut self) {
            let agora = agora_iso_completo();
            self.paused_at = None;
            if let Some(u) = self.pause_history.iter_mut().rev().find(|p| p.resumed_at.is_none()) {
                u.resumed_at = Some(agora);
            }
        }

        // Método do diagrama UML: +finalizar() (RF10)
        fn finalizar(&mut self) {
            let agora = agora_iso_completo();
            self.finished_at = Some(agora.clone());
            self.paused_at = None;
            if let Some(u) = self.pause_history.iter_mut().rev().find(|p| p.resumed_at.is_none()) {
                u.resumed_at = Some(agora);
            }
        }

        fn dentro_de_pausa(&self, scheduled_dt: &str) -> bool {
            let Some(ep_sched) = datetime_iso_para_epoch(scheduled_dt) else {
                return false;
            };
            for p in &self.pause_history {
                if let Some(ep_ini) = datetime_iso_para_epoch(&p.paused_at) {
                    let ep_fim = p.resumed_at.as_deref().and_then(datetime_iso_para_epoch);
                    if ep_sched >= ep_ini && ep_fim.map_or(true, |f| ep_sched < f) {
                        return true;
                    }
                }
            }
            false
        }

        fn data_devida(&self, data_alvo: &str) -> bool {
            if data_alvo < self.data_inicio.as_str() {
                return false;
            }
            for ocorrencia in range(1001) {
                if let Some(data_oc) = adicionar_intervalo_tratamento(
                    &self.data_inicio,
                    &self.interval_unit,
                    self.interval_value,
                    ocorrencia,
                ) {
                    if data_oc.as_str() > data_alvo {
                        return false;
                    }
                    if data_oc == data_alvo {
                        return true;
                    }
                } else {
                    break;
                }
            }
            false
        }

        fn contar_doses_planejadas(&self) -> Option<usize> {
            let fim = self.data_fim.as_ref()?.as_str();
            if fim.is_empty() {
                return None;
            }
            let mut total = 0usize;
            for ocorrencia in range(1001) {
                let Some(data_oc) = adicionar_intervalo_tratamento(
                    &self.data_inicio,
                    &self.interval_unit,
                    self.interval_value,
                    ocorrencia,
                ) else {
                    break;
                };
                if data_oc.as_str() > fim {
                    break;
                }
                for horario in &self.times {
                    if self.disabled_times.contains(horario) {
                        continue;
                    }
                    let dt = format!("{data_oc}T{horario}:00");
                    if let Some(ref fin_at) = self.finished_at {
                        if dt.as_str() > fin_at.as_str() {
                            continue;
                        }
                    }
                    if !self.dentro_de_pausa(&dt) {
                        total += 1;
                    }
                }
            }
            Some(total)
        }

        // Método do diagrama UML: +gerarDoses() (RF11, RN11)
        fn gerar_doses(&self, hoje: &str) -> Vec<String> {
            let mut agendamentos = Vec::new();
            if self.paused_at.is_some() || self.finished_at.is_some() {
                return agendamentos;
            }
            for offset in range(30) {
                let data_alvo = somar_dias_iso(hoje, offset);
                if let Some(ref fim) = self.data_fim {
                    if !fim.is_empty() && data_alvo.as_str() > fim.as_str() {
                        continue;
                    }
                }
                if !self.data_devida(&data_alvo) {
                    continue;
                }
                for horario in &self.times {
                    if self.disabled_times.contains(horario) {
                        continue;
                    }
                    let dt = format!("{data_alvo}T{horario}:00");
                    if !self.dentro_de_pausa(&dt) {
                        agendamentos.append_item(dt);
                    }
                }
            }
            agendamentos
        }
    }
}

// Herança: Dose herda de EntidadeFamiliar (`id` e `family_id`)
class! {
    struct Dose : EntidadeFamiliar {
        treatment_id: Option<String>,
        pet_id: String,
        medication_id: String,
        data_hora_prevista: String,
        data_hora_administrada: Option<String>,
        administered_by: Option<String>,
        status: String,
        notified: bool,
    }
    impl {
        // Método do diagrama UML: +marcarAdministrada(usuario, dataHora) (RF13, RN13)
        fn marcar_administrada(&mut self, usuario_id: String, data_hora: String) {
            self.status = "done".to_string();
            self.administered_by = Some(usuario_id);
            self.data_hora_administrada = Some(data_hora);
        }

        // Método do diagrama UML: +desfazerAdministracao() (RF13)
        fn desfazer_administracao(&mut self) {
            self.status = "pending".to_string();
            self.administered_by = None;
            self.data_hora_administrada = None;
            self.notified = false;
        }
    }
}
