use kinema_domain::{
    analyze_meeting, AtwoodMachine, Body, FreeBodyDiagram, FrictionState, MeetingInstant,
    MeetingOutcome, Motion, Motion1D, ParametricLaw, ParticleRope, Scene, TablePulleySystem,
};
use kinema_ports::SnapshotSink;

/// Type of kinematic graph displayed in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphKind {
    #[default]
    PositionTime,
    VelocityTime,
    AccelerationTime,
}

/// UI representation of a single body in the inspector and scene.
#[derive(Debug, Clone, PartialEq)]
pub struct UiBodyView {
    pub id: String,
    pub name: String,
    pub x0: f64,
    pub v0: f64,
    pub a: f64,
    pub current_position: f64,
    pub current_velocity: f64,
    pub current_acceleration: f64,
    pub formula_text: String,
    pub stopping_time: Option<f64>,
}

/// Meeting marker rendered on the timeline and graphs.
#[derive(Debug, Clone, PartialEq)]
pub struct UiMeetingMarker {
    pub time: f64,
    pub position: f64,
    pub is_past: bool,
    pub label: String,
}

/// Stopping instant marker (v = 0).
#[derive(Debug, Clone, PartialEq)]
pub struct UiStoppingMarker {
    pub body_id: String,
    pub time: f64,
    pub position: f64,
    pub label: String,
}

/// Apex marker for free vertical motion (t_up, h_max).
#[derive(Debug, Clone, PartialEq)]
pub struct UiApexMarker {
    pub body_id: String,
    pub time: f64,
    pub height: f64,
    pub label: String,
}

/// Ground impact marker for free vertical motion (t_impact, |v_impact|).
#[derive(Debug, Clone, PartialEq)]
pub struct UiImpactMarker {
    pub body_id: String,
    pub time: f64,
    pub speed: f64,
    pub label: String,
}

/// Arrow representing a single force in the Free-Body Diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct UiFbdArrow {
    pub label: String,
    pub magnitude: f64,
    pub direction: String,
}

/// Free-Body Diagram view for the UI inspector.
#[derive(Debug, Clone, PartialEq)]
pub struct UiFbdView {
    pub body_id: String,
    pub friction_state: String,
    pub net_force: f64,
    pub weight: f64,
    pub normal: f64,
    pub friction: f64,
    pub applied_force: f64,
    pub gravity_parallel: f64,
    pub arrows: Vec<UiFbdArrow>,
}

/// View of an ideal pulley system (Atwood machine or table pulley).
#[derive(Debug, Clone, PartialEq)]
pub struct UiPulleyView {
    pub body_id: String,
    pub system_type: String,
    pub tension: f64,
    pub acceleration: f64,
    pub mass1_pos: f64,
    pub mass2_pos: f64,
    pub status_text: String,
}

/// Visual segment along a particle-chain rope with tension color mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct UiRopeSegmentView {
    pub p0: [f64; 2],
    pub p1: [f64; 2],
    pub tension: f64,
    pub tension_ratio: f64,
    pub color_hex: String,
}

/// Canvas representation of a particle-chain rope with color ramp along segments.
#[derive(Debug, Clone, PartialEq)]
pub struct UiRopeView {
    pub rope_id: String,
    pub node_count: usize,
    pub total_length: f64,
    pub stretch_percent: f64,
    pub nodes: Vec<[f64; 2]>,
    pub segments: Vec<UiRopeSegmentView>,
}

/// A series of points for plotting a kinematic line curve.
#[derive(Debug, Clone, PartialEq)]
pub struct UiGraphSeries {
    pub body_id: String,
    pub body_name: String,
    pub points: Vec<(f64, f64)>,
}

/// Theme palette configuration according to Section 5.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiTheme {
    #[default]
    Classic,
    Phosphor,
    Amber,
}

impl UiTheme {
    pub fn palette(&self) -> UiColorPalette {
        match self {
            UiTheme::Classic => default_classic_palette(),
            UiTheme::Phosphor => default_phosphor_palette(),
            UiTheme::Amber => default_amber_palette(),
        }
    }
}

/// Palette tokens for UI windows, bevels, canvases, and vectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiColorPalette {
    pub window_face: String,
    pub bevel_light: String,
    pub bevel_dark: String,
    pub title_bg: String,
    pub title_fg: String,
    pub canvas_bg: String,
    pub canvas_grid: String,
    pub body_a: String,
    pub body_b: String,
    pub vector_velocity: String,
    pub vector_acceleration: String,
    pub vector_force: String,
    pub text_primary: String,
}

/// Help topics available in the help system (Section 5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpTopic {
    #[default]
    Contents,
    EquationReference,
    About,
}

impl HelpTopic {
    pub fn title(&self) -> &'static str {
        match self {
            HelpTopic::Contents => "KINEMA Help Contents (F1)",
            HelpTopic::EquationReference => "Equation Reference",
            HelpTopic::About => "About KINEMA",
        }
    }

    pub fn content(&self) -> &'static str {
        match self {
            HelpTopic::Contents => {
                "KINEMA Desktop Physics Workbench\n\n\
                 Keyboard Shortcuts:\n\
                   Space      Play / Pause simulation\n\
                   Right      Step forward (1/60 s)\n\
                   Left       Step backward (1/60 s)\n\
                   Home       Reset to t = 0\n\
                   Ctrl+Z     Undo last parameter edit\n\
                   Ctrl+Y     Redo last parameter edit\n\
                   F1         Open Help Contents\n\
                   Ctrl+E     Export scene as PNG image\n\n\
                 Mouse Interaction:\n\
                   Drag body to change initial position x0.\n\
                   Drag arrow head to mutate initial velocity or applied force."
            }
            HelpTopic::EquationReference => {
                "KINEMA Equation Catalog:\n\n\
                 M1 - MRU:            x(t) = x0 + v·t, a = 0\n\
                 M2 - MRUV:           x(t) = x0 + v0·t + ½·a·t², v(t) = v0 + a·t\n\
                 M3 - Free Fall:      y(t) = y0 + v0·t - ½·g·t², v(t) = v0 - g·t\n\
                 M4 - Friction:       F_net = F_app - mg·sin(θ) - f_roz, f_roz = μ·N\n\
                 M5 - Atwood Machine: a = (m2 - m1)·g / (m1 + m2), T = 2·m1·m2·g / (m1 + m2)\n\
                 M5 - Table Pulley:   a = (m2·g - μ_k·m1·g) / (m1 + m2), T = m1·(a + μ_k·g)\n\
                 M5 - Particle Rope:  Verlet x_{n+1} = 2·x_n - x_{n-1} + a·dt², 12 relaxation passes"
            }
            HelpTopic::About => {
                "KINEMA - Interactive Desktop Physics Workbench\n\
                 Version 0.1.0 (Revision 0.1)\n\n\
                 Design Philosophy: 'Old but functional'\n\
                 Architecture: Pure Hexagonal Core + Adapters\n\
                 Engineered with Rust, zero warnings, CC <= 5, and SQALE clean architecture.\n\
                 License: MIT / Apache-2.0"
            }
        }
    }
}

/// Single item in the application menu bar.
#[derive(Debug, Clone, PartialEq)]
pub struct UiMenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub action_id: String,
}

/// Menu tree node for classic desktop menu bar.
#[derive(Debug, Clone, PartialEq)]
pub struct UiMenu {
    pub title: String,
    pub items: Vec<UiMenuItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiViewModel {
    pub window_title: String,
    pub current_time: f64,
    pub bodies: Vec<UiBodyView>,
    pub meeting_diagnosis: String,
    pub meeting_markers: Vec<UiMeetingMarker>,
    pub stopping_markers: Vec<UiStoppingMarker>,
    pub apex_markers: Vec<UiApexMarker>,
    pub impact_markers: Vec<UiImpactMarker>,
    pub active_graph_kind: GraphKind,
    pub graph_series: Vec<UiGraphSeries>,
    pub fbd_views: Vec<UiFbdView>,
    pub pulley_views: Vec<UiPulleyView>,
    pub rope_views: Vec<UiRopeView>,
    pub active_theme: UiTheme,
    pub palette: UiColorPalette,
    pub menus: Vec<UiMenu>,
    pub help_dialog_open: bool,
    pub current_help_topic: Option<HelpTopic>,
    pub help_text: Option<String>,
    pub status_message: String,
}

impl Default for UiViewModel {
    fn default() -> Self {
        Self {
            window_title: "KINEMA - [Interactive Physics Workbench]".to_string(),
            current_time: 0.0,
            bodies: Vec::new(),
            meeting_diagnosis: "No bodies loaded".to_string(),
            meeting_markers: Vec::new(),
            stopping_markers: Vec::new(),
            apex_markers: Vec::new(),
            impact_markers: Vec::new(),
            active_graph_kind: GraphKind::PositionTime,
            graph_series: Vec::new(),
            fbd_views: Vec::new(),
            pulley_views: Vec::new(),
            rope_views: Vec::new(),
            active_theme: UiTheme::Classic,
            palette: default_classic_palette(),
            menus: build_default_menu_bar(),
            help_dialog_open: false,
            current_help_topic: None,
            help_text: None,
            status_message: "Ready.".to_string(),
        }
    }
}

pub struct UiPresenter {
    model: UiViewModel,
    theme: UiTheme,
}

impl UiPresenter {
    pub fn new() -> Self {
        Self {
            model: UiViewModel::default(),
            theme: UiTheme::Classic,
        }
    }
}

impl Default for UiPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl UiPresenter {
    pub fn model(&self) -> &UiViewModel {
        &self.model
    }

    pub fn set_status(&mut self, msg: &str) {
        self.model.status_message = msg.to_string();
    }

    pub fn set_graph_kind(&mut self, kind: GraphKind) {
        self.model.active_graph_kind = kind;
    }

    pub fn theme(&self) -> UiTheme {
        self.theme
    }

    pub fn set_theme(&mut self, theme: UiTheme) {
        self.theme = theme;
        self.model.active_theme = theme;
        self.model.palette = theme.palette();
    }

    pub fn open_help(&mut self, topic: HelpTopic) {
        self.model.help_dialog_open = true;
        self.model.current_help_topic = Some(topic);
        self.model.help_text = Some(topic.content().to_string());
    }

    pub fn close_help(&mut self) {
        self.model.help_dialog_open = false;
        self.model.current_help_topic = None;
        self.model.help_text = None;
    }
}

impl SnapshotSink for UiPresenter {
    fn consume_snapshot(&mut self, scene: &Scene, current_time: f64) {
        self.model.window_title = format!("KINEMA - [{}]", scene.name);
        self.model.current_time = current_time;
        self.model.active_theme = self.theme;
        self.model.palette = self.theme.palette();
        self.model.bodies = build_body_views(&scene.bodies, current_time);

        let (diag, markers) = compute_meeting_analysis(&scene.bodies);
        self.model.meeting_diagnosis = diag;
        self.model.meeting_markers = markers;
        self.model.stopping_markers = build_stopping_markers(&scene.bodies);
        self.model.apex_markers = build_apex_markers(&scene.bodies);
        self.model.impact_markers = build_impact_markers(&scene.bodies);
        self.model.fbd_views = build_fbd_views(&scene.bodies, current_time);
        self.model.pulley_views = build_pulley_views(&scene.bodies, current_time);
        self.model.rope_views = build_rope_views(&scene.ropes);

        let t_max = compute_graph_t_max(&self.model.meeting_markers, &self.model.impact_markers);
        self.model.graph_series =
            build_graph_series(&scene.bodies, self.model.active_graph_kind, t_max);

        self.model.status_message = format!("Scene: '{}' | t = {:.2}s", scene.name, current_time);
    }
}

fn build_body_views(bodies: &[Body], current_time: f64) -> Vec<UiBodyView> {
    bodies
        .iter()
        .map(|b| UiBodyView {
            id: b.id.clone(),
            name: b.name.clone(),
            x0: b.motion.position_at(0.0),
            v0: b.motion.velocity_at(0.0),
            a: b.motion.acceleration_at(0.0),
            current_position: b.motion.position_at(current_time),
            current_velocity: b.motion.velocity_at(current_time),
            current_acceleration: b.motion.acceleration_at(current_time),
            formula_text: b.motion.formula_text(),
            stopping_time: b.motion.stopping_time(),
        })
        .collect()
}

fn compute_meeting_analysis(bodies: &[Body]) -> (String, Vec<UiMeetingMarker>) {
    if bodies.len() < 2 {
        return (
            "Insufficient bodies for meeting analysis".to_string(),
            Vec::new(),
        );
    }

    let outcome = analyze_meeting(&bodies[0].motion, &bodies[1].motion);
    let msg = outcome.diagnostic_message().to_string();
    let markers = match outcome {
        MeetingOutcome::Single(inst) => vec![make_marker(inst)],
        MeetingOutcome::Dual(i1, i2) => vec![make_marker(i1), make_marker(i2)],
        _ => Vec::new(),
    };

    (msg, markers)
}

fn build_stopping_markers(bodies: &[Body]) -> Vec<UiStoppingMarker> {
    bodies
        .iter()
        .filter_map(|b| {
            b.motion.stopping_time().map(|ts| UiStoppingMarker {
                body_id: b.id.clone(),
                time: ts,
                position: b.motion.position_at(ts),
                label: format!(
                    "stopping ({} at t={:.2}s, x={:.2}m)",
                    b.name,
                    ts,
                    b.motion.position_at(ts)
                ),
            })
        })
        .collect()
}

fn build_apex_markers(bodies: &[Body]) -> Vec<UiApexMarker> {
    bodies
        .iter()
        .filter_map(|b| {
            b.motion.apex().map(|(t_up, h_max)| UiApexMarker {
                body_id: b.id.clone(),
                time: t_up,
                height: h_max,
                label: format!("apex ({} at t={:.2}s, y={:.2}m)", b.name, t_up, h_max),
            })
        })
        .collect()
}

fn build_impact_markers(bodies: &[Body]) -> Vec<UiImpactMarker> {
    bodies
        .iter()
        .filter_map(|b| {
            b.motion.ground_impact().map(|(t_i, v_i)| UiImpactMarker {
                body_id: b.id.clone(),
                time: t_i,
                speed: v_i,
                label: format!("impact ({} at t={:.2}s, |v|={:.2}m/s)", b.name, t_i, v_i),
            })
        })
        .collect()
}

fn build_fbd_views(bodies: &[Body], current_time: f64) -> Vec<UiFbdView> {
    bodies
        .iter()
        .filter_map(|b| {
            let v = b.motion.velocity_at(current_time);
            b.motion.free_body_diagram(v).map(|fbd| {
                let state_str = match fbd.state {
                    FrictionState::Static => "STATIC",
                    FrictionState::Kinetic => "KINETIC",
                };
                let arrows = make_fbd_arrows(&fbd);
                UiFbdView {
                    body_id: b.id.clone(),
                    friction_state: state_str.to_string(),
                    net_force: fbd.net_force,
                    weight: fbd.weight,
                    normal: fbd.normal,
                    friction: fbd.friction,
                    applied_force: fbd.applied_force,
                    gravity_parallel: fbd.gravity_parallel,
                    arrows,
                }
            })
        })
        .collect()
}

fn make_fbd_arrows(fbd: &FreeBodyDiagram) -> Vec<UiFbdArrow> {
    vec![
        UiFbdArrow {
            label: format!("W = {:.2} N", fbd.weight),
            magnitude: fbd.weight,
            direction: "downward".to_string(),
        },
        UiFbdArrow {
            label: format!("N = {:.2} N", fbd.normal),
            magnitude: fbd.normal,
            direction: "normal to surface".to_string(),
        },
        UiFbdArrow {
            label: format!("f_roz = {:.2} N", fbd.friction.abs()),
            magnitude: fbd.friction.abs(),
            direction: if fbd.friction >= 0.0 {
                "up-slope"
            } else {
                "down-slope"
            }
            .to_string(),
        },
        UiFbdArrow {
            label: format!("F_app = {:.2} N", fbd.applied_force.abs()),
            magnitude: fbd.applied_force.abs(),
            direction: if fbd.applied_force >= 0.0 {
                "up-slope"
            } else {
                "down-slope"
            }
            .to_string(),
        },
    ]
}

fn compute_graph_t_max(
    meeting_markers: &[UiMeetingMarker],
    impact_markers: &[UiImpactMarker],
) -> f64 {
    impact_markers
        .last()
        .map(|m| m.time * 1.2)
        .or_else(|| meeting_markers.last().map(|m| m.time.abs() * 1.5))
        .unwrap_or(10.0)
        .max(10.0)
}

fn make_marker(inst: MeetingInstant) -> UiMeetingMarker {
    let label = if inst.is_past {
        format!("past (t={:.2}s, x={:.2}m)", inst.time, inst.position)
    } else {
        format!("meeting (t={:.2}s, x={:.2}m)", inst.time, inst.position)
    };
    UiMeetingMarker {
        time: inst.time,
        position: inst.position,
        is_past: inst.is_past,
        label,
    }
}

fn build_graph_series(bodies: &[Body], kind: GraphKind, t_end: f64) -> Vec<UiGraphSeries> {
    bodies
        .iter()
        .map(|b| {
            let points = (0..=10)
                .map(|i| {
                    let t = (t_end * i as f64) / 10.0;
                    let val = match kind {
                        GraphKind::PositionTime => b.motion.position_at(t),
                        GraphKind::VelocityTime => b.motion.velocity_at(t),
                        GraphKind::AccelerationTime => b.motion.acceleration_at(t),
                    };
                    (t, val)
                })
                .collect();
            UiGraphSeries {
                body_id: b.id.clone(),
                body_name: b.name.clone(),
                points,
            }
        })
        .collect()
}

fn build_pulley_views(bodies: &[Body], current_time: f64) -> Vec<UiPulleyView> {
    bodies
        .iter()
        .filter_map(|b| make_single_pulley_view(b, current_time))
        .collect()
}

fn make_single_pulley_view(body: &Body, current_time: f64) -> Option<UiPulleyView> {
    match &body.motion {
        Motion::Atwood(a) => Some(make_atwood_view(body, a, current_time)),
        Motion::TablePulley(p) => Some(make_table_pulley_view(body, p, current_time)),
        _ => None,
    }
}

fn make_atwood_view(body: &Body, a: &AtwoodMachine, current_time: f64) -> UiPulleyView {
    UiPulleyView {
        body_id: body.id.clone(),
        system_type: "Atwood Machine".to_string(),
        tension: a.tension(),
        acceleration: a.acceleration(),
        mass1_pos: a.mass1_position_at(current_time),
        mass2_pos: a.mass2_position_at(current_time),
        status_text: format!(
            "m1={:.2}kg, m2={:.2}kg | a={:.3}m/s² | T={:.2}N",
            a.m1,
            a.m2,
            a.acceleration(),
            a.tension()
        ),
    }
}

fn make_table_pulley_view(body: &Body, p: &TablePulleySystem, current_time: f64) -> UiPulleyView {
    UiPulleyView {
        body_id: body.id.clone(),
        system_type: "Table Pulley".to_string(),
        tension: p.tension(),
        acceleration: p.acceleration(),
        mass1_pos: p.position_at(current_time),
        mass2_pos: -p.position_at(current_time),
        status_text: format!(
            "m1={:.2}kg, m2={:.2}kg | state={:?} | a={:.3}m/s² | T={:.2}N",
            p.m1,
            p.m2,
            p.friction_state(),
            p.acceleration(),
            p.tension()
        ),
    }
}

fn build_rope_views(ropes: &[ParticleRope]) -> Vec<UiRopeView> {
    ropes
        .iter()
        .enumerate()
        .map(make_single_rope_view)
        .collect()
}

fn make_single_rope_view((idx, rope): (usize, &ParticleRope)) -> UiRopeView {
    let dt = 1.0 / 240.0;
    let tensions = rope.segment_tensions(dt);
    let max_t = tensions.iter().copied().fold(1.0_f64, f64::max);
    let segments = build_rope_segments(rope, &tensions, max_t);

    UiRopeView {
        rope_id: format!("rope_{}", idx + 1),
        node_count: rope.nodes.len(),
        total_length: rope.total_length,
        stretch_percent: rope.stretch_ratio() * 100.0,
        nodes: rope.nodes.iter().map(|n| n.pos).collect(),
        segments,
    }
}

fn build_rope_segments(
    rope: &ParticleRope,
    tensions: &[f64],
    max_t: f64,
) -> Vec<UiRopeSegmentView> {
    tensions
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            let ratio = (t / max_t).clamp(0.0, 1.0);
            UiRopeSegmentView {
                p0: rope.nodes[i].pos,
                p1: rope.nodes[i + 1].pos,
                tension: t,
                tension_ratio: ratio,
                color_hex: tension_to_color_hex(ratio),
            }
        })
        .collect()
}

/// Maps normalized tension [0.0, 1.0] to a hex RGB color ramp from blue to red.
pub fn tension_to_color_hex(ratio: f64) -> String {
    let r = ratio.clamp(0.0, 1.0);
    let (red, green, blue) = if r < 0.33 {
        let t = r / 0.33;
        (0.0, t * 255.0, (1.0 - t) * 255.0)
    } else if r < 0.66 {
        let t = (r - 0.33) / 0.33;
        (t * 255.0, 255.0, 0.0)
    } else {
        let t = (r - 0.66) / 0.34;
        (255.0, (1.0 - t) * 255.0, 0.0)
    };
    format!("#{:02X}{:02X}{:02X}", red as u8, green as u8, blue as u8)
}

fn default_classic_palette() -> UiColorPalette {
    UiColorPalette {
        window_face: "#C0C0C0".to_string(),
        bevel_light: "#FFFFFF".to_string(),
        bevel_dark: "#808080".to_string(),
        title_bg: "#000080".to_string(),
        title_fg: "#FFFFFF".to_string(),
        canvas_bg: "#000000".to_string(),
        canvas_grid: "#555555".to_string(),
        body_a: "#FF5555".to_string(),
        body_b: "#55FFFF".to_string(),
        vector_velocity: "#55FF55".to_string(),
        vector_acceleration: "#FFFF55".to_string(),
        vector_force: "#FF55FF".to_string(),
        text_primary: "#000000".to_string(),
    }
}

fn default_phosphor_palette() -> UiColorPalette {
    UiColorPalette {
        window_face: "#000000".to_string(),
        bevel_light: "#33FF33".to_string(),
        bevel_dark: "#115511".to_string(),
        title_bg: "#113311".to_string(),
        title_fg: "#33FF33".to_string(),
        canvas_bg: "#000000".to_string(),
        canvas_grid: "#115511".to_string(),
        body_a: "#33FF33".to_string(),
        body_b: "#66FF66".to_string(),
        vector_velocity: "#33FF33".to_string(),
        vector_acceleration: "#88FF88".to_string(),
        vector_force: "#33FF33".to_string(),
        text_primary: "#33FF33".to_string(),
    }
}

fn default_amber_palette() -> UiColorPalette {
    UiColorPalette {
        window_face: "#000000".to_string(),
        bevel_light: "#FFB000".to_string(),
        bevel_dark: "#664400".to_string(),
        title_bg: "#332200".to_string(),
        title_fg: "#FFB000".to_string(),
        canvas_bg: "#000000".to_string(),
        canvas_grid: "#664400".to_string(),
        body_a: "#FFB000".to_string(),
        body_b: "#FFCC44".to_string(),
        vector_velocity: "#FFB000".to_string(),
        vector_acceleration: "#FFD066".to_string(),
        vector_force: "#FFB000".to_string(),
        text_primary: "#FFB000".to_string(),
    }
}

fn make_menu(title: &str, items: &[(&str, Option<&str>, &str)]) -> UiMenu {
    UiMenu {
        title: title.to_string(),
        items: items
            .iter()
            .map(|(label, shortcut, action)| UiMenuItem {
                label: label.to_string(),
                shortcut: shortcut.map(|s| s.to_string()),
                action_id: action.to_string(),
            })
            .collect(),
    }
}

fn build_default_menu_bar() -> Vec<UiMenu> {
    vec![
        make_file_menu(),
        make_edit_menu(),
        make_view_menu(),
        make_simulate_menu(),
        make_scene_menu(),
        make_help_menu(),
    ]
}

fn make_file_menu() -> UiMenu {
    make_menu(
        "File",
        &[
            ("New", Some("Ctrl+N"), "file.new"),
            ("Open...", Some("Ctrl+O"), "file.open"),
            ("Save", Some("Ctrl+S"), "file.save"),
            ("Save As...", None, "file.save_as"),
            ("Export Image (PNG)", Some("Ctrl+E"), "file.export_png"),
            ("Exit", Some("Alt+F4"), "file.exit"),
        ],
    )
}

fn make_edit_menu() -> UiMenu {
    make_menu(
        "Edit",
        &[
            ("Undo", Some("Ctrl+Z"), "edit.undo"),
            ("Redo", Some("Ctrl+Y"), "edit.redo"),
            ("Duplicate Body", Some("Ctrl+D"), "edit.duplicate"),
            ("Delete Body", Some("Del"), "edit.delete"),
            ("Preferences...", None, "edit.preferences"),
        ],
    )
}

fn make_view_menu() -> UiMenu {
    make_menu(
        "View",
        &[
            ("Equation Panel", None, "view.equations"),
            ("Graph Panel", None, "view.graphs"),
            ("Inspector", None, "view.inspector"),
            ("Theme: Classic", None, "theme.classic"),
            ("Theme: Phosphor", None, "theme.phosphor"),
            ("Theme: Amber", None, "theme.amber"),
        ],
    )
}

fn make_simulate_menu() -> UiMenu {
    make_menu(
        "Simulate",
        &[
            ("Play / Pause", Some("Space"), "sim.play_pause"),
            ("Step Forward", Some("Right"), "sim.step_fwd"),
            ("Step Backward", Some("Left"), "sim.step_back"),
            ("Reset", Some("Home"), "sim.reset"),
        ],
    )
}

fn make_scene_menu() -> UiMenu {
    make_menu(
        "Scene",
        &[
            ("Add Vehicle", None, "scene.add_vehicle"),
            ("Add Falling Object", None, "scene.add_falling"),
            ("Add Block", None, "scene.add_block"),
            ("Add Surface / Incline", None, "scene.add_incline"),
            ("Add Rope", None, "scene.add_rope"),
            ("Add Pulley", None, "scene.add_pulley"),
        ],
    )
}

fn make_help_menu() -> UiMenu {
    make_menu(
        "Help",
        &[
            ("Contents (F1)", Some("F1"), "help.contents"),
            ("Equation Reference", None, "help.equations"),
            ("About KINEMA", None, "help.about"),
        ],
    )
}

