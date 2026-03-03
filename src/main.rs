mod app;
mod gl_utils;
mod highlight;
mod shader;
mod theme;

use app::App;

fn main() -> eframe::Result {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
            ..Default::default()
        };
        eframe::run_native(
            "GLSL Notebook",
            options,
            Box::new(|cc| Ok(Box::new(App::new(cc)))),
        )
    }

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        wasm_bindgen_futures::spawn_local(async {
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("the_canvas_id")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();

            eframe::WebRunner::new()
                .start(
                    canvas,
                    eframe::WebOptions::default(),
                    Box::new(|cc| Ok(Box::new(App::new(cc)))),
                )
                .await
                .expect("failed to start eframe");
        });
        Ok(())
    }
}
