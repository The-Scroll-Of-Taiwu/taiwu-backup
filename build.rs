// build task tray resource

use windres::Build;

fn main() {
    Build::new().compile("assets/tray.rc").unwrap();
}
