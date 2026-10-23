<div align="center">
<img style="width:128px" src="examples/logo.svg" width="128px">

<a href="https://github.com/egakareta/ecss/actions/workflows/ci.yml"><img alt="Build Status" src="https://github.com/egakareta/ecss/actions/workflows/ci.yml/badge.svg?branch=master"/></a>
<a href="https://crates.io/crates/ecss"><img alt="crates.io" src="https://img.shields.io/crates/v/ecss"/></a>

</div>

# ecss 🐦

Add cascading style sheet (CSS) support to [egui](https://github.com/emilk/egui).

```rust,no_run
use ecss::{SelectorExt as _, Stylesheet};

fn ui(ui: &mut egui::Ui, context: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
    let css = Stylesheet::from_file("theme.css")?;
    css.install(context);
    ui.add(egui::Button::new("Save").selector(".primary"));

    Ok(())
}
```

> This library is not heavily tested and will contain bugs. Use at your own risk.

Please check out [examples](https://github.com/egakareta/ecss/tree/master/examples) to
learn more.

## Performance

This library may feel choppy even if you have a weak or powerful device. It's recommended to set
`opt-level` on egui and ecss to ensure a smoother development experience.

```toml
# on just ecss
[profile.dev.package.ecss]
opt-level = 3

# on all dependencies
[profile.dev.package."*"]
opt-level = 3
```

## License

Dual-licensed under the [MIT License](https://opensource.org/licenses/MIT) and the [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0). Choose whichever you prefer. This license only applies to ecss code.

Note that ecss is not affiliated with or endorsed by egui.
