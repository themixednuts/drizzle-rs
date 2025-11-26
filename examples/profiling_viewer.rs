use drizzle::prelude::*;
use eframe::egui;

#[derive(Default)]
struct ProfilerApp {}

impl ProfilerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        puffin::set_scopes_on(true);

        Self::default()
    }

    fn run_sql_tests(&self) {
        #[SQLiteTable(name = "users")]
        struct User {
            #[integer(primary)]
            id: i32,
            #[text]
            name: String,
        }

        #[derive(SQLiteSchema)]
        struct Schema {
            user: User,
        }

        let Schema { user } = Schema::new();
        let qb = drizzle_sqlite::builder::QueryBuilder::new::<Schema>();

        // Test SQL rendering with profiling
        for i in 0..100 {
            let query = qb
                .select((user.id, user.name))
                .from(user)
                .r#where(eq(user.id, i));

            // This will trigger sql_render profiling
            let _sql_string = query.to_sql().sql();
        }
    }
}

impl eframe::App for ProfilerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        puffin::GlobalProfiler::lock().new_frame();

        egui::SidePanel::left("controls").show(ctx, |ui| {
            ui.heading("Drizzle SQL Profiler");

            if ui.button("Run SQL Tests").clicked() {
                self.run_sql_tests();
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut profile = puffin::are_scopes_on();
            ui.checkbox(&mut profile, "Show profiler window");
            puffin::set_scopes_on(profile);
            puffin_egui::profiler_window(ctx);
            self.run_sql_tests();

            if ui.button("Quit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Drizzle Profiler",
        options,
        Box::new(|cc| Ok(Box::new(ProfilerApp::new(cc)))),
    )
}
