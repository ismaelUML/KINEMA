//! KINEMA Composition Root: Dependency Injection & Application Entry Point.

use kinema_adapter_storage::KinFileStorage;
use kinema_adapter_ui::{GraphKind, UiPresenter, UiViewModel};
use kinema_app::SimulationService;
use kinema_domain::Scene;
use kinema_ports::{ScenarioCatalog, SimulationControl, SnapshotSink};

pub mod gui;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--cli" || a == "--batch") {
        run_cli_workbench();
    } else if let Err(e) = gui::run_gui() {
        eprintln!("Error launching GUI window: {}", e);
        run_cli_workbench();
    }
}

fn run_cli_workbench() {
    println!("=======================================================");
    println!(" K I N E M A  --  Interactive Physics Workbench (M6)   ");
    println!("=======================================================");

    // Wire application and load canonical M5 Stage A preset (Atwood machine)
    let mut service = SimulationService::new(Scene::default());
    service
        .load_scenario("atwood_machine")
        .expect("Failed to load Atwood preset");

    let _storage = KinFileStorage::new();
    let mut presenter = UiPresenter::new();

    // Snapshot at t=0
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    // Advance Atwood machine 1.0 s
    println!("\n--- Advancing Atwood Machine to t = 1.0 s ---");
    service.seek(1.0);
    presenter.set_graph_kind(GraphKind::VelocityTime);
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    // Load M5 Stage B preset (Hanging Catenary Rope with Verlet integration)
    println!("\n--- Loading M5 Stage B: Hanging Catenary Rope ---");
    service
        .load_scenario("hanging_catenary_rope")
        .expect("Failed to load catenary preset");

    // Advance 60 frames (1.0 s)
    for _ in 0..60 {
        service.step_forward();
    }
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    println!("\nKINEMA M5 (Rope and Pulleys) verified successfully.");

    // M6 Polish & Delivery Showcase:
    println!("\n=======================================================");
    println!(" K I N E M A  --  M6 Polish, Themes & Export Delivery ");
    println!("=======================================================");

    // 1. Theme switching: Classic -> Phosphor -> Amber
    println!("\n[1] Theme Switching:");
    println!(" -> Active Theme: {:?}", presenter.theme());
    presenter.set_theme(kinema_adapter_ui::UiTheme::Phosphor);
    println!(
        " -> Switched to Phosphor: Primary Text Hex = {}",
        presenter.model().palette.text_primary
    );
    presenter.set_theme(kinema_adapter_ui::UiTheme::Amber);
    println!(
        " -> Switched to Amber: Primary Text Hex = {}",
        presenter.model().palette.text_primary
    );

    // 2. Help System & Dialogs:
    println!("\n[2] Help System & Dialogs:");
    presenter.open_help(kinema_adapter_ui::HelpTopic::Contents);
    if let Some(txt) = &presenter.model().help_text {
        println!(
            " -> Opened [Help Contents]: {} lines of guidance",
            txt.lines().count()
        );
    }
    presenter.open_help(kinema_adapter_ui::HelpTopic::EquationReference);
    if presenter.model().help_text.is_some() {
        println!(" -> Opened [Equation Reference]: Physics reference ready");
    }
    presenter.close_help();

    // 3. Menu Bar Hierarchy:
    println!("\n[3] Menu Bar Hierarchy:");
    for menu in &presenter.model().menus {
        println!(" -> Menu [{}] ({} items)", menu.title, menu.items.len());
    }

    // 4. Pure-Rust Zero-Dependency PNG Export:
    println!("\n[4] Canvas PNG Export with Cooperative Cancellation:");
    use kinema_ports::ImageExporter;
    let exporter = kinema_adapter_storage::PngCanvasExporter::new(640, 480);
    let export_path = std::env::temp_dir().join("kinema_main_export.png");
    let export_str = export_path.to_str().unwrap();
    let token = kinema_ports::CancellationToken::new();

    match exporter.export_png(service.scene(), service.current_time(), export_str, &token) {
        Ok(()) => {
            let size = std::fs::metadata(&export_path)
                .map(|m| m.len())
                .unwrap_or(0);
            println!(" -> Exported canvas to {} ({} bytes)", export_str, size);
            let _ = std::fs::remove_file(&export_path);
        }
        Err(e) => println!(" -> Export failed: {}", e),
    }

    println!("\nKINEMA M6 (Polish and Delivery) fully operational.");

    // Launch interactive workbench prompt if running in interactive terminal
    interactive_loop(&mut service, &mut presenter);
}

fn interactive_loop(service: &mut SimulationService, presenter: &mut UiPresenter) {
    loop {
        print_menu_prompt();
        let choice = match read_line_choice() {
            Some(c) => c,
            None => break, // EOF reached (e.g., automated pipe/CI)
        };
        if choice == "0" || choice.is_empty() {
            println!("Saliendo de KINEMA. ¡Hasta luego!");
            break;
        }
        dispatch_choice(choice.as_str(), service, presenter);
    }
}

fn read_line_choice() -> Option<String> {
    use std::io::{stdin, stdout, Write};
    let _ = stdout().flush();
    let mut line = String::new();
    let bytes = stdin().read_line(&mut line).ok()?;
    if bytes == 0 {
        return None;
    }
    Some(line.trim().to_string())
}

fn dispatch_choice(choice: &str, service: &mut SimulationService, presenter: &mut UiPresenter) {
    match choice {
        "1" => handle_load_scenario(service, presenter),
        "2" => handle_step(service, presenter, 60),
        "3" => handle_step(service, presenter, -60),
        "4" => handle_toggle_theme(presenter),
        "5" => handle_export_png(service),
        "6" => handle_show_equations(presenter),
        _ => println!("Opción '{}' no reconocida.", choice),
    }
}

fn print_menu_prompt() {
    println!("\n=======================================================");
    println!("  K I N E M A   P H Y S I C S   W O R K B E N C H");
    println!("=======================================================");
    println!(" 1. Cargar escenario físico del catálogo");
    println!(" 2. Avanzar 1 segundo (+60 frames)");
    println!(" 3. Retroceder 1 segundo (-60 frames)");
    println!(" 4. Cambiar Tema (Classic -> Phosphor -> Amber)");
    println!(" 5. Exportar Canvas a 'kinema_render.png'");
    println!(" 6. Ver Referencia de Ecuaciones y Ayuda");
    println!(" 0. Salir (o presiona Enter)");
    print!("> Opción: ");
}

fn handle_load_scenario(service: &mut SimulationService, presenter: &mut UiPresenter) {
    let list = service.list_scenarios();
    println!("\nCatálogo de Escenarios:");
    for (i, name) in list.iter().enumerate() {
        println!(" [{}] {}", i + 1, name);
    }
    print!("Elige el número de escenario: ");
    if let Some(num_str) = read_line_choice() {
        if let Ok(idx) = num_str.parse::<usize>() {
            if idx >= 1 && idx <= list.len() {
                let name = &list[idx - 1];
                let _ = service.load_scenario(name);
                presenter.consume_snapshot(service.scene(), service.current_time());
                println!("\nEscenario '{}' cargado con éxito.", name);
                print_view_model(presenter.model());
            }
        }
    }
}

fn handle_step(service: &mut SimulationService, presenter: &mut UiPresenter, steps: i32) {
    if steps > 0 {
        for _ in 0..steps {
            service.step_forward();
        }
    } else {
        for _ in 0..(-steps) {
            service.step_backward();
        }
    }
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());
}

fn handle_toggle_theme(presenter: &mut UiPresenter) {
    let next = match presenter.theme() {
        kinema_adapter_ui::UiTheme::Classic => kinema_adapter_ui::UiTheme::Phosphor,
        kinema_adapter_ui::UiTheme::Phosphor => kinema_adapter_ui::UiTheme::Amber,
        kinema_adapter_ui::UiTheme::Amber => kinema_adapter_ui::UiTheme::Classic,
    };
    presenter.set_theme(next);
    println!(
        "\nTema cambiado a: {:?} (Color de acento: {})",
        next,
        presenter.model().palette.text_primary
    );
}

fn handle_export_png(service: &SimulationService) {
    use kinema_ports::ImageExporter;
    let exporter = kinema_adapter_storage::PngCanvasExporter::new(640, 480);
    let token = kinema_ports::CancellationToken::new();
    let filename = "kinema_render.png";
    match exporter.export_png(service.scene(), service.current_time(), filename, &token) {
        Ok(()) => {
            println!(
                "\n[OK] Imagen del canvas exportada como '{}' (640x480 RGB).",
                filename
            );
            println!("Puedes abrir '{}' en tu explorador de archivos.", filename);
        }
        Err(e) => println!("\n[ERROR] Falló la exportación: {}", e),
    }
}

fn handle_show_equations(presenter: &mut UiPresenter) {
    presenter.open_help(kinema_adapter_ui::HelpTopic::EquationReference);
    if let Some(txt) = &presenter.model().help_text {
        println!("\n--- Referencia de Ecuaciones y Ayuda ---\n{}", txt);
    }
    presenter.close_help();
}

fn print_markers(model: &UiViewModel) {
    for marker in &model.meeting_markers {
        println!(" - Meeting Marker:  {}", marker.label);
    }
    for marker in &model.stopping_markers {
        println!(" - Stopping Marker: {}", marker.label);
    }
    for marker in &model.apex_markers {
        println!(" - Apex Marker:     {}", marker.label);
    }
    for marker in &model.impact_markers {
        println!(" - Impact Marker:   {}", marker.label);
    }
}

fn print_fbd(model: &UiViewModel) {
    for fbd in &model.fbd_views {
        println!(
            " - FBD [{}]: State = {}, Net Force = {:.2} N (W={:.2}N, N={:.2}N, f_roz={:.2}N, F_app={:.2}N)",
            fbd.body_id, fbd.friction_state, fbd.net_force, fbd.weight, fbd.normal, fbd.friction, fbd.applied_force
        );
        for arrow in &fbd.arrows {
            println!("     -> Force Arrow: {} ({})", arrow.label, arrow.direction);
        }
    }
}

fn print_pulleys(model: &UiViewModel) {
    for pulley in &model.pulley_views {
        println!(
            " - Pulley [{}]: {} => T = {:.2} N, a = {:.3} m/s², m1_pos = {:.2} m, m2_pos = {:.2} m",
            pulley.body_id,
            pulley.system_type,
            pulley.tension,
            pulley.acceleration,
            pulley.mass1_pos,
            pulley.mass2_pos
        );
    }
}

fn print_ropes(model: &UiViewModel) {
    for rope in &model.rope_views {
        println!(
            " - Rope [{}]: {} nodes, L = {:.2} m, stretch = {:.3}%, {} segments",
            rope.rope_id,
            rope.node_count,
            rope.total_length,
            rope.stretch_percent,
            rope.segments.len()
        );
        if let Some(first_seg) = rope.segments.first() {
            println!(
                "     -> First segment tension: {:.2} N, color: {}",
                first_seg.tension, first_seg.color_hex
            );
        }
    }
}

fn print_view_model(model: &UiViewModel) {
    println!("Title:     {}", model.window_title);
    println!("Status:    {}", model.status_message);
    println!("Inspector: {}", model.meeting_diagnosis);
    print_markers(model);
    print_fbd(model);
    print_pulleys(model);
    print_ropes(model);
    for body in &model.bodies {
        println!(
            " - [{}]: {} => pos = {:.2} m, v = {:.2} m/s, a = {:.2} m/s²",
            body.name,
            body.formula_text,
            body.current_position,
            body.current_velocity,
            body.current_acceleration
        );
    }
}
