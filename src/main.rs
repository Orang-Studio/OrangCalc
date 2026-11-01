mod engine;
mod ui;
mod xml;
use gtk::prelude::*;

fn main() {
    engine::install_panic_hook();
    gtk::glib::log_set_writer_func(|level, fields| {
        let msg = fields.iter().find(|f| f.key() == "MESSAGE").and_then(|f| f.value_str()).unwrap_or("");
        if (level == gtk::glib::LogLevel::Warning || level == gtk::glib::LogLevel::Critical) && msg.contains("GtkGizmo") && msg.contains("sizes must be >= 0") {
            return gtk::glib::LogWriterOutput::Handled;
        }
        gtk::glib::log_writer_default(level, fields)
    });
    let args: Vec<String> = std::env::args().skip(1).collect();
    // orangcalc thing
    if args.first().map(|a| a == "-e").unwrap_or(false) {
        let c = engine::Calculator::create();
        for a in &args[1..] {
            let r = c.calculate(a, 10000, false);
            match (&r.error, &r.result) {
                (Some(e), _) => println!("{} => error: {}", a, e),
                (None, Some(v)) => println!("{} => {}", a, engine::formatter::format(v, &r.print).0),
                _ => println!("{} => (empty)", a),
            }
        }
        return;
    }
    let mut settings = ui::settings::AppSettings::default();
    let mut opts = engine::CalcOptions::default();
    ui::settings::load_settings(&mut settings, &mut opts);
    let flags = if settings.multiple_instances { gtk::gio::ApplicationFlags::NON_UNIQUE } else { gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE };
    let app = gtk::Application::builder().application_id("lt.oranges.OrangCalc").flags(flags).build();
    let main_window: std::rc::Rc<std::cell::RefCell<Option<std::rc::Rc<ui::app::App>>>> = Default::default();
    let mw = main_window.clone();
    app.connect_activate(move |gapp| {
        if let Some(a) = mw.borrow().as_ref() {
            a.window.present();
            return;
        }
        let a = ui::app::App::build(gapp);
        ui::theme::apply_fonts(&a.settings.borrow());
        *mw.borrow_mut() = Some(a);
    });
    let mw = main_window.clone();
    app.connect_command_line(move |gapp, cl| {
        gapp.activate();
        let text: Vec<String> = cl.arguments().iter().skip(1).map(|a| a.to_string_lossy().to_string()).collect();
        if !text.is_empty() {
            if let Some(a) = mw.borrow().as_ref() {
                a.insert_into_expression(&text.join(" "), true);
            }
        }
        gtk::glib::ExitCode::SUCCESS
    });
    app.run_with_args(&std::env::args().take(1).chain(args).collect::<Vec<_>>());
    drop(main_window);
}