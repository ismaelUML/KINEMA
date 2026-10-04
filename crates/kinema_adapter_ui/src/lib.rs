//! Driving/Presenter Adapter: UI State, rendering abstractions, and presenter sink.

use kinema_domain::{
    analyze_meeting, Body, MeetingInstant, MeetingOutcome, Motion1D, ParametricLaw, Scene,
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
    pub active_graph_kind: GraphKind,
    pub graph_series: Vec<UiGraphSeries>,
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
            active_graph_kind: GraphKind::PositionTime,
            graph_series: Vec::new(),
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

        let t_max = self
            .model
            .meeting_markers
            .last()
            .map(|m| m.time.abs() * 1.5)
            .unwrap_or(10.0)
            .max(10.0);
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
                label: format!("stopping ({} at t={:.2}s, x={:.2}m)", b.name, ts, b.motion.position_at(ts)),
            })
        })
        .collect()
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
