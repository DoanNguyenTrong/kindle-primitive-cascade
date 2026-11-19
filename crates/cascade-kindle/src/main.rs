use std::path::Path;

fn main() -> Result<(), String> {
    let dir = Path::new("/mnt/us/cascade");
    primitive_app::run(cascade_render::View::load(dir.join("game.json")), dir)
}
