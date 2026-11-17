use crate::domain::{NivelSeveridade, ResumoCartao};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TemaApp {
    Escuro,
    Claro,
}

pub struct PaletaCores {
    pub bg_main: Color32,
    pub bg_card: Color32,
    pub bg_nav: Color32,
    pub text_main: Color32,
    pub text_muted: Color32,
    pub primary: Color32,
    pub primary_light: Color32,
    pub on_primary: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub success: Color32,
    pub border: Color32,
}

impl PaletaCores {
    pub fn para_tema(tema: TemaApp) -> Self {
        match tema {
            TemaApp::Escuro => Self {
                bg_main: Color32::from_rgb(26, 33, 44),
                bg_card: Color32::from_rgb(38, 48, 64),
                bg_nav: Color32::from_rgb(32, 41, 56),
                text_main: Color32::from_rgb(241, 245, 249),
                text_muted: Color32::from_rgb(148, 163, 184),
                primary: Color32::from_rgb(92, 207, 189),
                primary_light: Color32::from_rgb(34, 64, 63),
                on_primary: Color32::from_rgb(14, 42, 39),
                warning: Color32::from_rgb(230, 126, 34),
                danger: Color32::from_rgb(231, 76, 60),
                success: Color32::from_rgb(46, 204, 113),
                border: Color32::from_rgb(53, 66, 90),
            },
            TemaApp::Claro => Self {
                bg_main: Color32::from_rgb(238, 244, 244),
                bg_card: Color32::from_rgb(255, 255, 255),
                bg_nav: Color32::from_rgb(255, 255, 255),
                text_main: Color32::from_rgb(31, 45, 58),
                text_muted: Color32::from_rgb(100, 116, 139),
                primary: Color32::from_rgb(28, 151, 146),
                primary_light: Color32::from_rgb(224, 244, 242),
                on_primary: Color32::from_rgb(255, 255, 255),
                warning: Color32::from_rgb(230, 126, 34),
                danger: Color32::from_rgb(231, 76, 60),
                success: Color32::from_rgb(46, 204, 113),
                border: Color32::from_rgb(214, 228, 228),
            },
        }
    }

    pub fn aplicar_no_contexto(&self, ctx: &egui::Context, tema: TemaApp) {
        let mut visuals = if tema == TemaApp::Escuro {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = self.bg_main;
        visuals.window_fill = self.bg_card;
        visuals.override_text_color = Some(self.text_main);
        visuals.widgets.noninteractive.bg_fill = self.bg_card;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border);
        visuals.widgets.inactive.bg_fill = self.bg_main;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.border);
        visuals.widgets.hovered.bg_fill = self.primary_light;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, self.primary);
        visuals.widgets.active.bg_fill = self.primary;
        visuals.selection.bg_fill = self.primary;
        visuals.selection.stroke = Stroke::new(1.0, self.on_primary);
        ctx.set_visuals(visuals);
    }

    pub fn cor_severidade(&self, sev: NivelSeveridade) -> Color32 {
        match sev {
            NivelSeveridade::Sucesso => self.success,
            NivelSeveridade::Aviso => self.warning,
            NivelSeveridade::Perigo => self.danger,
            NivelSeveridade::Neutro => self.text_muted,
        }
    }
}

pub fn card_frame(cores: &PaletaCores) -> egui::Frame {
    egui::Frame::NONE
        .fill(cores.bg_card)
        .stroke(Stroke::new(1.0, cores.border))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(egui::Margin::same(16))
}

pub fn botao_primario(ui: &mut egui::Ui, texto: &str, cores: &PaletaCores) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(texto)
                .color(cores.on_primary)
                .strong()
                .size(15.0),
        )
        .fill(cores.primary)
        .corner_radius(CornerRadius::same(24))
        .min_size(Vec2::new(ui.available_width(), 40.0)),
    )
}

pub fn pilula_aba(
    ui: &mut egui::Ui,
    rotulo: &str,
    ativo: bool,
    cores: &PaletaCores,
) -> egui::Response {
    let fill = if ativo {
        cores.primary_light
    } else {
        cores.bg_card
    };
    let stroke = if ativo {
        Stroke::new(1.5, cores.primary)
    } else {
        Stroke::new(1.0, cores.border)
    };
    let text_color = if ativo {
        cores.primary
    } else {
        cores.text_muted
    };
    ui.add(
        egui::Button::new(RichText::new(rotulo).color(text_color).strong().size(13.0))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(CornerRadius::same(20)),
    )
}

/// Renderiza o cabeçalho de qualquer entidade que implemente a trait polimórfica `ResumoCartao`.
pub fn renderizar_cabecalho_polimorfico<T: ResumoCartao>(
    ui: &mut egui::Ui,
    entidade: &T,
    hoje: &str,
    cores: &PaletaCores,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(entidade.icone_cartao()).size(26.0));
        ui.vertical(|ui| {
            if let Some((texto_selo, sev)) = entidade.selo_status(hoje) {
                ui.label(
                    RichText::new(format!(" {texto_selo} "))
                        .size(11.0)
                        .color(Color32::WHITE)
                        .background_color(cores.cor_severidade(sev)),
                );
            }
            ui.label(RichText::new(entidade.titulo_cartao()).size(17.5).strong());
        });
    });
    ui.label(RichText::new(entidade.subtitulo_cartao()).color(cores.text_muted));
}
