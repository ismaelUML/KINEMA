//! Guided interactive physics challenges and pedagogical criteria evaluation.

use crate::motion::Motion1D;
use crate::scene::Scene;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeCategory {
    Kinematics1D,
    Dynamics,
    FreeFall,
    Atwood,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChallengeCondition {
    /// Two bodies meet within time [min_time, max_time] and position [min_pos, max_pos].
    MeetingWithinRange {
        min_time: f64,
        max_time: f64,
        min_pos: f64,
        max_pos: f64,
    },
    /// A specified body comes to a complete halt within a target distance [min_distance, max_distance].
    StoppingDistanceWithin {
        body_id: String,
        min_distance: f64,
        max_distance: f64,
    },
    /// A specified body hits ground at time [min_time, max_time].
    GroundImpactTimeWithin {
        body_id: String,
        min_time: f64,
        max_time: f64,
    },
    /// A pulley/Atwood system achieves an acceleration within [min_acc, max_acc].
    AccelerationWithin {
        body_id: String,
        min_acc: f64,
        max_acc: f64,
    },
    /// Applied force overcomes static friction threshold and achieves a minimum net force.
    OvercomeStaticFriction {
        body_id: String,
        min_net_force: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChallengeStatus {
    InProgress {
        current_metric: String,
        progress_ratio: f32,
    },
    Solved {
        summary: String,
        stars: u8,
    },
    Failed {
        reason: String,
    },
}

impl ChallengeStatus {
    pub fn is_solved(&self) -> bool {
        matches!(self, ChallengeStatus::Solved { .. })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Challenge {
    pub id: String,
    pub title: String,
    pub category: ChallengeCategory,
    pub scenario_name: String,
    pub description: String,
    pub objective: String,
    pub hint: String,
    pub condition: ChallengeCondition,
}

impl Challenge {
    pub fn evaluate(&self, scene: &Scene) -> ChallengeStatus {
        match &self.condition {
            ChallengeCondition::MeetingWithinRange {
                min_time,
                max_time,
                min_pos,
                max_pos,
            } => evaluate_meeting(*min_time, *max_time, *min_pos, *max_pos, scene),
            ChallengeCondition::StoppingDistanceWithin {
                body_id,
                min_distance,
                max_distance,
            } => evaluate_stopping(body_id, *min_distance, *max_distance, scene),
            ChallengeCondition::GroundImpactTimeWithin {
                body_id,
                min_time,
                max_time,
            } => evaluate_ground_impact(body_id, *min_time, *max_time, scene),
            ChallengeCondition::AccelerationWithin {
                body_id,
                min_acc,
                max_acc,
            } => evaluate_acceleration(body_id, *min_acc, *max_acc, scene),
            ChallengeCondition::OvercomeStaticFriction {
                body_id,
                min_net_force,
            } => evaluate_friction(body_id, *min_net_force, scene),
        }
    }

    pub fn catalog() -> Vec<Self> {
        vec![
            Self {
                id: "challenge_mru_intercept".to_string(),
                title: "1. Intercepción de Tránsito (MRU)".to_string(),
                category: ChallengeCategory::Kinematics1D,
                scenario_name: "two_cars_mru".to_string(),
                description: "Un vehículo infractor avanza a velocidad constante. La patrulla debe interceptarlo antes de que escape de la zona urbana.".to_string(),
                objective: "Ajustá la velocidad de patrulla del Móvil A para interceptar al Móvil B en t ∈ [4.0, 5.0] s y x ∈ [50.0, 70.0] m.".to_string(),
                hint: "x_A(t) = x_{0A} + v_A t. En el encuentro: x_A(t) = x_B(t) => v_A = (x_{0B} - x_{0A} + v_B t) / t.".to_string(),
                condition: ChallengeCondition::MeetingWithinRange {
                    min_time: 4.0,
                    max_time: 5.0,
                    min_pos: 50.0,
                    max_pos: 70.0,
                },
            },
            Self {
                id: "challenge_mruv_braking".to_string(),
                title: "2. Frenado de Precisión (MRUV)".to_string(),
                category: ChallengeCategory::Kinematics1D,
                scenario_name: "two_cars_mruv".to_string(),
                description: "Un deportivo a alta velocidad se aproxima a una zona de obras y debe detenerse por completo justo antes del perímetro de seguridad.".to_string(),
                objective: "Ajustá la desaceleración del auto para frenar recorriendo entre 38.0m y 42.0m (meta: 40m).".to_string(),
                hint: "Ecuación de Torricelli: v^2 = v_0^2 + 2 a d. Al detenerse v = 0 => d = v_0^2 / (2 |a|).".to_string(),
                condition: ChallengeCondition::StoppingDistanceWithin {
                    body_id: "car2".to_string(),
                    min_distance: 38.0,
                    max_distance: 42.0,
                },
            },
            Self {
                id: "challenge_moon_drop".to_string(),
                title: "3. Caída Libre en la Luna (Apolo 15)".to_string(),
                category: ChallengeCategory::FreeFall,
                scenario_name: "feather_and_hammer_moon".to_string(),
                description: "En el vacío lunar sin resistencia del aire, calculá la altura exacta para sincronizar el impacto contra el regolito lunar.".to_string(),
                objective: "Ajustá la altura inicial h en la Luna (g = 1.62 m/s²) para que el martillo toque el suelo entre 4.8s y 5.2s.".to_string(),
                hint: "Caída libre desde reposo: h = 0.5 * g * t^2. Para t = 5.0s: h = 0.5 * 1.62 * 25.0 = 20.25 m.".to_string(),
                condition: ChallengeCondition::GroundImpactTimeWithin {
                    body_id: "hammer".to_string(),
                    min_time: 4.8,
                    max_time: 5.2,
                },
            },
            Self {
                id: "challenge_atwood_acc".to_string(),
                title: "4. Contrapeso de Atwood".to_string(),
                category: ChallengeCategory::Atwood,
                scenario_name: "atwood_machine".to_string(),
                description: "Diseñá un ascensor contrapesado de laboratorio ajustando la masa de tracción para lograr un régimen de aceleración suave.".to_string(),
                objective: "Con masa m1 = 10.0 kg fija, ajustá m2 para lograr una aceleración del sistema a ∈ [1.9, 2.1] m/s².".to_string(),
                hint: "a = (m2 - m1) / (m1 + m2) * g. Despejando: m2 = m1 * (g + a) / (g - a). Con g = 9.81 y a = 2 => m2 ≈ 15.1 kg.".to_string(),
                condition: ChallengeCondition::AccelerationWithin {
                    body_id: "atwood_sys".to_string(),
                    min_acc: 1.9,
                    max_acc: 2.1,
                },
            },
            Self {
                id: "challenge_coulomb_friction".to_string(),
                title: "5. Venciendo el Rozamiento Estático".to_string(),
                category: ChallengeCategory::Dynamics,
                scenario_name: "block_friction_threshold".to_string(),
                description: "Una pesada caja industrial de 50 kg descansa sobre hormigón. Aplicá la fuerza horizontal justa para romper la fricción.".to_string(),
                objective: "Con m = 50 kg y mu_s = 0.5, aplicá suficiente f_app para superar el umbral estático y lograr F_net ≥ 5.0 N.".to_string(),
                hint: "F_est_max = mu_s * m * g = 0.5 * 50 * 9.81 = 245.25 N. Si f_app > 245.25 N, el bloque desliza con fricción cinética.".to_string(),
                condition: ChallengeCondition::OvercomeStaticFriction {
                    body_id: "block".to_string(),
                    min_net_force: 5.0,
                },
            },
        ]
    }
}

fn in_range(val: f64, min: f64, max: f64) -> bool {
    val >= min && val <= max
}

fn evaluate_meeting(
    min_t: f64,
    max_t: f64,
    min_x: f64,
    max_x: f64,
    scene: &Scene,
) -> ChallengeStatus {
    if scene.bodies.len() < 2 {
        return ChallengeStatus::Failed {
            reason: "Se requieren al menos 2 móviles para el desafío de encuentro.".to_string(),
        };
    }
    let outcome = crate::meeting::analyze_meeting(&scene.bodies[0].motion, &scene.bodies[1].motion);
    match outcome {
        crate::meeting::MeetingOutcome::Single(m)
        | crate::meeting::MeetingOutcome::Dual(m, _) => {
            if in_range(m.time, min_t, max_t) && in_range(m.position, min_x, max_x) {
                ChallengeStatus::Solved {
                    summary: format!(
                        "¡Intercepción perfecta en t = {:.2}s, x = {:.1}m!",
                        m.time, m.position
                    ),
                    stars: 3,
                }
            } else {
                ChallengeStatus::InProgress {
                    current_metric: format!(
                        "Encuentro en t = {:.2}s, x = {:.1}m (Objetivo: t ∈ [{:.1}, {:.1}], x ∈ [{:.1}, {:.1}])",
                        m.time, m.position, min_t, max_t, min_x, max_x
                    ),
                    progress_ratio: 0.5,
                }
            }
        }
        _ => ChallengeStatus::InProgress {
            current_metric:
                "Los móviles no se cruzan. Ajustá la velocidad o posición inicial.".to_string(),
            progress_ratio: 0.1,
        },
    }
}

fn evaluate_stopping(
    body_id: &str,
    min_d: f64,
    max_d: f64,
    scene: &Scene,
) -> ChallengeStatus {
    let body = match scene.bodies.iter().find(|b| b.id == body_id) {
        Some(b) => b,
        None => {
            return ChallengeStatus::Failed {
                reason: format!("Móvil '{}' no encontrado en la escena.", body_id),
            }
        }
    };
    let t_stop = match body.motion.stopping_time() {
        Some(ts) => ts,
        None => {
            return ChallengeStatus::InProgress {
                current_metric: "El móvil no tiene aceleración negativa o no frena.".to_string(),
                progress_ratio: 0.1,
            }
        }
    };
    let x_stop = body.motion.position_at(t_stop);
    let x0 = body.motion.position_at(0.0);
    let distance = (x_stop - x0).abs();
    if in_range(distance, min_d, max_d) {
        ChallengeStatus::Solved {
            summary: format!(
                "¡Frenado preciso a distancia d = {:.2}m en t = {:.2}s!",
                distance, t_stop
            ),
            stars: 3,
        }
    } else {
        ChallengeStatus::InProgress {
            current_metric: format!(
                "Distancia de frenado actual: {:.2}m (Objetivo: [{:.1}..{:.1}]m)",
                distance, min_d, max_d
            ),
            progress_ratio: 0.5,
        }
    }
}

fn evaluate_ground_impact(
    body_id: &str,
    min_t: f64,
    max_t: f64,
    scene: &Scene,
) -> ChallengeStatus {
    let body = match scene.bodies.iter().find(|b| b.id == body_id) {
        Some(b) => b,
        None => {
            return ChallengeStatus::Failed {
                reason: format!("Móvil '{}' no encontrado en la escena.", body_id),
            }
        }
    };
    match body.motion.ground_impact() {
        Some((t, _)) if in_range(t, min_t, max_t) => ChallengeStatus::Solved {
            summary: format!("¡Impacto contra el suelo a los {:.2}s exactos!", t),
            stars: 3,
        },
        Some((t, _)) => ChallengeStatus::InProgress {
            current_metric: format!(
                "Tiempo de impacto: {:.2}s (Objetivo: [{:.1}..{:.1}]s)",
                t, min_t, max_t
            ),
            progress_ratio: 0.5,
        },
        None => ChallengeStatus::InProgress {
            current_metric: "El cuerpo no llega al suelo con los parámetros actuales.".to_string(),
            progress_ratio: 0.1,
        },
    }
}

fn evaluate_acceleration(
    body_id: &str,
    min_a: f64,
    max_a: f64,
    scene: &Scene,
) -> ChallengeStatus {
    let body = match scene.bodies.iter().find(|b| b.id == body_id) {
        Some(b) => b,
        None => {
            return ChallengeStatus::Failed {
                reason: format!("Sistema '{}' no encontrado en la escena.", body_id),
            }
        }
    };
    let acc = body.motion.acceleration_at(0.0).abs();
    if in_range(acc, min_a, max_a) {
        ChallengeStatus::Solved {
            summary: format!("¡Aceleración objetivo alcanzada: {:.2} m/s²!", acc),
            stars: 3,
        }
    } else {
        ChallengeStatus::InProgress {
            current_metric: format!(
                "Aceleración actual: {:.2} m/s² (Objetivo: [{:.2}..{:.2}] m/s²)",
                acc, min_a, max_a
            ),
            progress_ratio: 0.5,
        }
    }
}

fn evaluate_friction(body_id: &str, min_net_force: f64, scene: &Scene) -> ChallengeStatus {
    let body = match scene.bodies.iter().find(|b| b.id == body_id) {
        Some(b) => b,
        None => {
            return ChallengeStatus::Failed {
                reason: format!("Cuerpo '{}' no encontrado en la escena.", body_id),
            }
        }
    };
    let net_f = match &body.motion {
        crate::motion::Motion::Dynamics(d) => d.free_body_diagram(0.0).net_force,
        _ => 0.0,
    };
    if net_f >= min_net_force {
        ChallengeStatus::Solved {
            summary: format!("¡Fuerza estática superada con F_net = {:.1} N!", net_f),
            stars: 3,
        }
    } else {
        ChallengeStatus::InProgress {
            current_metric: format!(
                "Fuerza neta actual: {:.1} N (Umbral requerido: ≥ {:.1} N). El cuerpo permanece inmóvil.",
                net_f, min_net_force
            ),
            progress_ratio: 0.3,
        }
    }
}
