# KINEMA

<p align="center">
  <strong>Interactive 2D Physics Studio & Workbench</strong><br>
  <em>High-precision deterministic physics engine, analytical problem solver, and procedural 2D visualization environment built in Rust with Hexagonal Architecture.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2021_Edition-orange?logo=rust&logoColor=white" alt="Rust 2021">
  <img src="https://img.shields.io/badge/Architecture-Hexagonal_Ports_%26_Adapters-blue" alt="Hexagonal Architecture">
  <img src="https://img.shields.io/badge/GUI-egui_%2F_eframe-teal" alt="GUI eframe">
  <img src="https://img.shields.io/badge/Quality_Gate-SonarCloud-brightgreen?logo=sonarcloud" alt="SonarCloud Quality Gate">
  <img src="https://img.shields.io/badge/Tests-81_Passing-success" alt="Tests 81 Passing">
  <img src="https://img.shields.io/badge/License-MIT_%2F_Apache--2.0-blue.svg" alt="License">
</p>

---

## 1. Overview

**KINEMA** is an open-source, interactive 2D physics simulation environment, educational problem solver, and workbench. Designed with the precision of classic physics instrumentation and the extensibility of a modern CAD studio, KINEMA evolves the concept of educational simulators into an interactive **"Blender for Physics"**.

Instead of treating simulation entities as abstract 1D scalars, KINEMA implements physically realistic 2D mechanics:
- **Vehicles with State Machines:** True braking models that stop at $v = 0$ rather than reversing into unbounded quadratic acceleration.
- **Ballistics & Free Fall:** Vertical Cartesian coordinate frames, apex detection ($\dot{y} = 0$), and ground impact restitution.
- **Frictional Dynamics:** Real-time static friction thresholds ($f_{s,\max} = \mu_s N$), kinetic sliding ($f_k = \mu_k N$), stick-slip zero-crossing halts, and inclined plane force decompositions.
- **Coupled Multi-Body Topologies:** Ideal Atwood machines, table-edge pulley systems, and catenary ropes resolved via position-based Verlet integration with thermal tension heatmaps.
- **Dual Runtime Interfaces:** A responsive Native Desktop GUI (`egui`/`eframe`) and a Headless CLI / Interactive Console Workbench for batch verification and CI.

---

## 2. Key Physical Capabilities & Models

### 2.1 Kinematics & Analytical Meeting Solver
- **Uniform Motion (MRU):** $x(t) = x_0 + v t$.
- **Accelerated Motion (MRUV):** $x(t) = x_0 + v_0 t + \frac{1}{2} a t^2$.
- **Physical Braking Logic:** Vehicle state machine (`Cruising`, `Braking`, `AtRest`) prevents unphysical quadratic reversal once speed reaches zero under braking friction.
- **Quadratic Meeting Solver:** Evaluates discriminant $\Delta = (v_A - v_B)^2 - 2(a_A - a_B)(x_A - x_B)$, diagnosing simultaneous coincidence, overtaking events, collision points, or divergent trajectories.
- **Live Kinematic Curves:** Real-time sampling and plotting of $x(t)$ (Position-Time), $v(t)$ (Velocity-Time), and $a(t)$ (Acceleration-Time).

### 2.2 Free Fall, Projectiles & Celestial Gravitation
- **Vertical Free Fall (MVL):** $y(t) = y_0 - \frac{1}{2} g t^2$ with automatic ground impact halt ($y \le 0$).
- **Vertical Ballistic Launch:** Apex detection ($t_{\text{up}} = \frac{v_0}{g}$, $h_{\max} = y_0 + \frac{v_0^2}{2g}$) and vector flipping at peak altitude.
- **Galileo Lunar Drop Experiment:** Simultaneous vacuum drop simulation of a falcon feather and an Apollo geological hammer ($g = 1.62\text{ m/s}^2$).
- **Gravity Presets:**
  - Earth Standard: $g = 9.80665\text{ m/s}^2$
  - Moon: $g = 1.62\text{ m/s}^2$
  - Mars: $g = 3.72\text{ m/s}^2$
  - Jupiter: $g = 24.79\text{ m/s}^2$
  - Zero-G: $g = 0.0\text{ m/s}^2$

### 2.3 Dynamics, Friction & Free-Body Diagrams (FBD)
- **Horizontal Sliding Blocks & Crates:** Evaluates applied push force $\vec{F}_{\text{ext}}$ against maximum static friction $f_{s,\max} = \mu_s N$. Dynamic transition to sliding acceleration $a = \frac{F_{\text{ext}} - \mu_k N}{m}$.
- **Stick-Slip Transition:** Velocity zero-crossing detection halts moving blocks when forces drop below static equilibrium.
- **Inclined Plane / Wedge Dynamics:** Tilted reference frame $(x_\parallel, y_\perp)$ with angle $\theta$:
  $$W_\parallel = m g \sin\theta, \quad W_\perp = m g \cos\theta, \quad N = m g \cos\theta$$
  Automatic detection of the critical angle $\theta_c = \arctan(\mu_s)$.
- **Real-Time FBD Overlays:** Dynamic vector arrows rendered directly on bodies: normal force $\vec{N}$, weight $\vec{W}$, applied force $\vec{F}_{\text{ext}}$, and friction $\vec{f}_r$.

### 2.4 Coupled Multi-Body Systems & Pulleys
- **Atwood Machine:** Analytical and Semi-Implicit Euler integration:
  $$a = \frac{m_2 - m_1}{m_1 + m_2} g, \quad T = \frac{2 m_1 m_2}{m_1 + m_2} g$$
- **Table Pulley System:** Coupled horizontal sliding block $m_1$ on tabletop with corner pulley and suspended hanging mass $m_2$:
  $$a = \frac{m_2 - \mu_k m_1}{m_1 + m_2} g \quad (\text{if } m_2 g > \mu_s m_1 g)$$
- **Particle Rope & Catenary Sag:** Position-based Verlet integration with relaxation constraints, rigid ground collisions, surface friction, and tension color ramp (Blue = slack, Green = nominal, Yellow = elevated, Red = high tension).

---

## 3. Architecture & Engineering Standards

KINEMA is implemented following strict **Hexagonal Architecture (Ports and Adapters)** to enforce total decoupling between domain mathematical algorithms, user interfaces, and file storage.

```mermaid
graph TD
    subgraph Driving Adapters
        GUI["apps/kinema (egui/eframe GUI)"]
        CLI["crates/kinema_adapter_cli"]
    end

    subgraph Ports Layer (kinema_ports)
        InPorts["Input Ports: SimulationControl, SceneEditing, ScenarioCatalog"]
        OutPorts["Output Ports: SnapshotSink, SceneRepository, ImageExporter"]
    end

    subgraph Application Layer (kinema_app)
        SimService["SimulationService (Undo/Redo, Timelines, Sampling)"]
    end

    subgraph Pure Domain Core (kinema_domain)
        Physics["Kinematics (MRU, MRUV, MVL)"]
        Dynamics["BlockDynamics & Inclined Wedge"]
        Pulleys["AtwoodMachine & TablePulley"]
        Ropes["ParticleRope (Verlet Integration)"]
        Meeting["Analytical Meeting Solver"]
    end

    subgraph Driven Adapters
        Storage["crates/kinema_adapter_storage (.kin Storage & PNG Exporter)"]
        UIAdapter["crates/kinema_adapter_ui (Presenter & ViewModels)"]
    end

    GUI --> InPorts
    CLI --> InPorts
    InPorts --> SimService
    SimService --> Physics
    SimService --> Dynamics
    SimService --> Pulleys
    SimService --> Ropes
    SimService --> Meeting
    SimService --> OutPorts
    OutPorts --> Storage
    OutPorts --> UIAdapter
```

### Workspace Structure

| Package / Crate | Role | Dependencies |
|---|---|---|
| [`crates/kinema_domain`](file:///crates/kinema_domain) | Pure mathematical models, physical equations, solvers, and Verlet engines. | **Zero external dependencies** |
| [`crates/kinema_ports`](file:///crates/kinema_ports) | Trait contracts for driving and driven boundaries (`SimulationControl`, `SceneEditing`, `ImageExporter`, etc.). | `kinema_domain` |
| [`crates/kinema_app`](file:///crates/kinema_app) | Use-case orchestration, 200-level bounded undo/redo history, trajectory curve samplers, and scenario catalog. | `kinema_domain`, `kinema_ports` |
| [`crates/kinema_adapter_ui`](file:///crates/kinema_adapter_ui) | UI Presenter, view models, FBD calculations, markers, and theme palettes. | `kinema_domain`, `kinema_ports` |
| [`crates/kinema_adapter_cli`](file:///crates/kinema_adapter_cli) | Driving adapter for headless terminal execution and text summaries. | `kinema_domain` |
| [`crates/kinema_adapter_storage`](file:///crates/kinema_adapter_storage) | Scene `.kin` parser/serializer and pure-Rust PNG canvas rasterizer with cooperative cancellation. | `kinema_domain`, `kinema_ports` |
| [`apps/kinema`](file:///apps/kinema) | Desktop executable composition root, `eframe`/`egui` desktop application, procedural vector renderers, and CLI fallback. | All workspace crates + `eframe` |

### SQALE & Code Quality Guarantees
- **Cognitive Complexity:** Enforced at $\le 5$ per function via `clippy.toml` (`cognitive-complexity-threshold = 5`).
- **Deterministic Math:** Float comparisons bounded by strict $\varepsilon$-tolerances; zero `NaN` propagation under numerical fuzzing.
- **CI & Quality Gates:** Automated SonarCloud pipeline with `cargo-llvm-cov` integration testing all 81 acceptance criteria on every pull request.

---

## 4. User Interface & Themes

KINEMA features procedural vector canvas renderers designed to replicate classic high-precision physics instrumentation:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ KINEMA - Interactive Physics Workbench [two_cars_mruv]               [_][#][X]│
├──────────────────────────────────────────────────────────────────────────────┤
│ File  Edit  View  Simulate  Scene  Help                                      │
├──────────────────────────────────────────────────────────────────────────────┤
│ [Play/Pause] [|< Reset] [<< -1s] [>> +1s]   Speed: [1.0x]   t = 5.000 s      │
├──────────────────────────────────────────────────┬───────────────────────────┤
│ 2D SCENE CANVAS                                  │ PHYSICAL INSPECTOR        │
│                                                  │ ------------------------- │
│  Lane 1:  [Car A] ===>                           │ CAR A: MRU                │
│  Lane 2:                 <=== [Car B (Braking)]  │  x = 75.0 m, v = 15.0 m/s │
│                                                  │ CAR B: MRUV (Braking)     │
│  ==============================================  │  x = 75.0 m, v = 0.0 m/s  │
│  0m        25m        50m        75m       100m  │ ------------------------- │
│                       ▲ Meeting Marker           │ MEETING STATUS:           │
│                                                  │  Collision at t = 5.00 s  │
│ - - - - - - - - - - - - - - - - - - - - - - - -  │ ------------------------- │
│ TRAJECTORY GRAPH: Position-Time [x-t]            │ ACTIVE THEME:             │
│                                                  │  [Phosphor Green (CRT)]   │
├──────────────────────────────────────────────────┴───────────────────────────┤
│ Ready. Simulation running at 60 FPS | 2 bodies | dt = 1/240 s                │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Aesthetic Themes
1. **Classic (Win95 Technical):** Crisp, high-contrast engineering palette reminiscent of 90s CAD and physics lab software.
2. **Phosphor Green (CRT):** Monochromatic 550nm green glow inspired by vintage laboratory oscilloscopes and terminal workstations.
3. **Amber Plasma (CRT):** Warm 590nm monochromatic phosphor terminal styling for low-fatigue nighttime analysis.

---

## 5. Built-in Preset Scenarios

KINEMA ships with 14 curated physical scenarios accessible from the menu or CLI:

| Key / Identifier | Scenario Name | Physical Phenomenon Demonstrated |
|---|---|---|
| `two_cars_mru` | Two Cars Meeting (MRU) | Opposing constant velocities; single linear meeting point. |
| `two_cars_mruv` | Two Cars Meeting (MRU vs MRUV) | Quadratic meeting with vehicle braking state ($v=0$ stop). |
| `parallel_cars` | Parallel Cars Never Meeting | Identical speeds in separate lanes; discriminant $\Delta < 0$. |
| `coinciding_cars` | Coinciding Cars | Identical initial states rendered with multi-lane separation. |
| `20m_free_fall` | Free Fall 20m Drop (Earth) | Terrestrial gravity $g=9.81\text{ m/s}^2$ with ground impact stop. |
| `feather_and_hammer_moon` | Feather & Hammer (Moon) | Vacuum drop in lunar gravity ($g=1.62\text{ m/s}^2$) hitting ground together. |
| `vertical_projectile` | Vertical Projectile Launch | Upward launch ($v_0 = 20\text{ m/s}$), apex detection, velocity flip. |
| `block_friction_threshold` | Block with Friction Threshold | Static vs kinetic friction threshold gauge ($F_{\text{ext}}$ vs $\mu_s N$). |
| `incline_plane_slide` | Incline Plane Sliding Angle | 30° ramp with gravity decomposition and critical angle check. |
| `heavy_crate_push` | Heavy Crate Push | 50 kg wooden crate with contact friction and force vectors. |
| `atwood_machine` | Atwood Machine | Ideal pulley with masses $2\text{ kg}$ vs $3\text{ kg}$; string tension. |
| `table_pulley_friction` | Table Pulley with Friction | Coupled sliding horizontal block and hanging vertical mass. |
| `hanging_catenary_rope` | Hanging Catenary Rope | 24-node Verlet rope sagging under gravity with tension coloring. |
| `rope_surface_friction` | Rope on Surface with Friction | Cable colliding with flat ground floor under Coulomb friction. |

---

## 6. Getting Started

### Prerequisites
- [Rust toolchain](https://www.rust-lang.org/tools/install) (version 1.75 or newer, 2021 Edition).

### Build from Source
```bash
# Clone the repository
git clone https://github.com/ismaelUML/KINEMA.git
cd KINEMA

# Build the entire workspace in release mode
cargo build --release --workspace
```

### Running the Native Desktop GUI
Launch the full interactive graphical environment:
```bash
cargo run --release
# or explicitly target the app crate:
cargo run --release -p kinema
```

### Running the Headless CLI & Interactive Workbench
For terminal-only environments, SSH sessions, or automated verification:
```bash
# Interactive terminal prompt:
cargo run --release -- --cli

# Automated batch summary verification:
cargo run --release -- --batch
```

---

## 7. Controls & Keyboard Shortcuts

| Shortcut | Action | Description |
|---|---|---|
| <kbd>Space</kbd> | **Play / Pause** | Toggles real-time physics integration. |
| <kbd>→</kbd> | **Step Forward** | Advances simulation by one frame ($+\frac{1}{60}\text{ s}$). |
| <kbd>←</kbd> | **Step Backward** | Rewinds simulation by one frame ($-\frac{1}{60}\text{ s}$). |
| <kbd>Home</kbd> | **Reset Time** | Rewinds time back to $t = 0.00\text{ s}$. |
| <kbd>Ctrl</kbd> + <kbd>Z</kbd> | **Undo** | Reverts parameter edits or scene mutations (up to 200 levels). |
| <kbd>Ctrl</kbd> + <kbd>Y</kbd> | **Redo** | Restores previously undone modifications. |
| <kbd>Ctrl</kbd> + <kbd>E</kbd> | **Export PNG** | Exports the current 2D canvas to a pure-Rust PNG image. |
| <kbd>F1</kbd> | **Help** | Opens the in-app interactive reference manual. |

---

## 8. Scene File Format (`.kin`)

KINEMA utilizes a clean, human-readable declarative format (`.kin`) for scene persistence:

```ini
# KINEMA scene file - format 1
[scene]
name = "Atwood Machine Example"
gravity = 9.80665

[body.atwood_rig]
kind = "atwood"
motion = "atwood"
m1 = 2.0
m2 = 3.0
x0 = 0.0
v = 0.0

[rope.catenary]
length = 2.5
mass = 1.0
nodes = 24
passes = 8
p0_x = 0.0
p0_y = 3.0
p1_x = 2.0
p1_y = 3.0
p1_pinned = true
surface_y = 0.0
friction_mu = 0.4
```

Files are validated with strict bounds checks (mass $> 0$, friction $\ge 0$, finite non-NaN coordinates) to guarantee engine stability.

---

## 9. Quality Assurance & Testing

KINEMA is engineered with comprehensive test coverage spanning unit, numerical, and integration tests:

```bash
# Run the complete test suite (81 tests)
cargo test --workspace

# Validate code formatting
cargo fmt --all --check

# Run strict Clippy quality rules (CC <= 5 target)
cargo clippy --workspace --all-targets -- -D warnings

# Generate test coverage report
cargo llvm-cov --workspace --lcov --output-path lcov.info
```

---

## 10. Strategic Roadmap

KINEMA is actively evolving from a specialized kinematic viewer into a comprehensive 2D physics creation suite. Refer to the [Strategic Roadmap](file:///docs/KINEMA_PHYSICS_EDITOR_ROADMAP.md) for complete details:
- **Phase 1 (Completed):** Physical Reality & Entity Typology (dedicated visual models, braking state machines, multi-body topologies).
- **Phase 2 (In Progress):** Interactive 2D Editor ("Blender for Physics") with scene graph, interactive gizmos, drag-and-drop tools, and parameter inspectors.
- **Phase 3:** Symbolic & Numerical Equation Engine (dynamic formula derivation and step-by-step educational proofs).
- **Phase 4:** High-DPI Export Studio (vector SVG and video rendering).
- **Phase 5:** 3D Viewport Expansion.

---

## 11. License

This project is dual-licensed under either:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
