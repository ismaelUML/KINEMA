use kinema_domain::{
    analyze_meeting, Body, FreeBodyDiagram, FrictionState, MeetingInstant, MeetingOutcome,
    Motion1D, ParametricLaw, Scene,
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

/// A series of points for plotting a kinematic line curve.
#[derive(Debug, Clone, PartialEq)]
pub struct UiGraphSeries {
    pub body_id: String,
    pub body_name: String,
    pub points: Vec<(f64, f64)>,
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
            status_message: "Ready.".to_string(),
        }
    }
}

pub struct UiPresenter {
    model: UiViewModel,
}

impl UiPresenter {
    pub fn new() -> Self {
        Self {
            model: UiViewModel::default(),
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
}

impl SnapshotSink for UiPresenter {
    fn consume_snapshot(&mut self, scene: &Scene, current_time: f64) {
        self.model.window_title = format!("KINEMA - [{}]", scene.name);
        self.model.current_time = current_time;
        self.model.bodies = build_body_views(&scene.bodies, current_time);

        let (diag, markers) = compute_meeting_analysis(&scene.bodies);
        self.model.meeting_diagnosis = diag;
        self.model.meeting_markers = markers;
        self.model.stopping_markers = build_stopping_markers(&scene.bodies);
        self.model.apex_markers = build_apex_markers(&scene.bodies);
        self.model.impact_markers = build_impact_markers(&scene.bodies);
        self.model.fbd_views = build_fbd_views(&scene.bodies, current_time);

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
