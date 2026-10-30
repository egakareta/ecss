use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn nonfinite_math_is_resolved_at_the_outer_calculation_boundary() {
    for (expression, expected) in [
        ("min(10px, calc(NaN * 1px))", 0.0),
        ("max(calc(NaN * 1px), 10px)", 0.0),
        ("clamp(5px, calc(NaN * 1px), 20px)", 0.0),
        ("calc(10px + min(5px, calc(NaN * 1px)))", 0.0),
        ("calc(pow(NaN, 0) * 10px)", 0.0),
        ("round(up, 10px, -3px)", 12.0),
        ("round(down, 10px, -3px)", 9.0),
        ("calc(10px + round(10px, calc(infinity * 1px)))", 10.0),
        ("mod(10px, calc(infinity * 1px))", 10.0),
        ("clamp(0px, calc(1px / -0), 20px)", 20.0),
        ("clamp(0px, calc(1px / round(-0, infinity)), 20px)", 20.0),
        ("clamp(0px, calc(1px / mod(-0, infinity)), 20px)", 20.0),
        ("calc(hypot(3e30px, 4e30px) / 1e30)", 5.0),
        ("clamp(0px, calc(infinity * 1px), 20px)", 20.0),
    ] {
        let context = Context::default();
        Stylesheet::parse(&format!(".box {{ width:{expression}; height:10px; }}"))
            .unwrap()
            .install(&context);
        let mut rect = Rect::NOTHING;
        frame(&context, 200.0, 0.0, Vec::new(), |ui| {
            rect = ui.add(empty_response.selector(".box")).rect;
        });
        assert!(rect.is_finite(), "{expression}: {rect:?}");
        assert!(
            (rect.width() - expected).abs() < 0.01,
            "{expression}: {rect:?}, expected width {expected}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn transitions_interpolate_censored_math_values_without_nonfinite_geometry() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { width:calc(NaN * 1px); height:20px; transition:width 1s linear; } \
         .box.changed { width:40px; }",
    )
    .unwrap()
    .install(&context);
    let changed = std::cell::Cell::new(false);
    let bounds = std::cell::Cell::new(Rect::NOTHING);
    let mut build = |ui: &mut Ui| {
        bounds.set(
            ui.add(empty_response.selector(if changed.get() {
                ".box.changed"
            } else {
                ".box"
            }))
            .rect,
        );
    };
    frame(&context, 200.0, 0.0, vec![], &mut build);
    changed.set(true);
    frame(&context, 200.0, 1.0, vec![], &mut build);
    frame(&context, 200.0, 1.5, vec![], &mut build);
    assert!(bounds.get().is_finite());
    assert!(
        (bounds.get().width() - 20.0).abs() < 0.05,
        "{:?}",
        bounds.get()
    );
    frame(&context, 200.0, 2.0, vec![], &mut build);
    assert_eq!(bounds.get().size(), vec2(40.0, 20.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn arithmetic_requires_a_math_function_at_the_property_boundary() {
    for declaration in [
        "width:calc(10px) + 5px",
        "width:10px + calc(5px)",
        "width:(10px)",
        "opacity:calc(.5) * 2",
        "flex-grow:(2)",
        "width:calc(10px +)",
        "width:calc()",
        "width:clamp(none, none, 10px)",
    ] {
        assert!(
            Stylesheet::parse(&format!(".box {{{declaration}}}")).is_err(),
            "{declaration}"
        );
        assert!(
            ecss::InlineStyle::parse(declaration).is_err(),
            "{declaration}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn excessive_math_complexity_returns_an_error_without_losing_ordinary_calculations() {
    let nested = format!("{}1px{}", "calc(".repeat(4096), ")".repeat(4096));
    let broad = format!("calc({})", vec!["1px"; 4096].join(" + "));
    for expression in [nested, broad] {
        assert!(Stylesheet::parse(&format!(".box {{width:{expression}}}")).is_err());
        assert!(ecss::InlineStyle::parse(&format!("width:{expression}")).is_err());
    }
    let context = Context::default();
    let expression = format!("calc({})", vec!["1px"; 64].join(" + "));
    Stylesheet::parse(&format!(".box {{ width:{expression}; height:10px; }}"))
        .unwrap()
        .install(&context);
    let mut rect = Rect::NOTHING;
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        rect = ui.add(empty_response.selector(".box")).rect;
    });
    assert_eq!(rect.size(), vec2(64.0, 10.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn numeric_math_censors_nonfinite_flex_factors_before_layout() {
    for (value, expected) in [
        ("NaN", 0.0),
        ("infinity", 100.0),
        ("-infinity", 0.0),
        ("1 / -0", 100.0),
    ] {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".row {{ display:flex; width:100px; }} \
             .first {{ flex:calc({value}) 0 0px; height:10px; }} \
             .second {{ flex:1 0 0px; height:10px; }}"
        ))
        .unwrap()
        .install(&context);
        let mut width = 0.0;
        frame(&context, 200.0, 0.0, vec![], |ui| {
            ui.add(
                (|ui: &mut Ui| {
                    width = ui.add(empty_response.selector(".first")).rect.width();
                    ui.add(empty_response.selector(".second"));
                    ui.response()
                })
                .selector(".row"),
            );
        });
        assert!((width - expected).abs() < 0.01, "{value}: {width}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn transitions_bound_large_percentage_calculations_after_resolving_the_basis() {
    for percentage in ["1e15%", "1e38%"] {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".box {{ width:100%; height:20px; transition:width 1s linear; }} \
             .box.changed {{ width:calc({percentage} + 0%); }}"
        ))
        .unwrap()
        .install(&context);
        let changed = std::cell::Cell::new(false);
        let bounds = std::cell::Cell::new(Rect::NOTHING);
        let mut build = |ui: &mut Ui| {
            bounds.set(
                ui.add(empty_response.selector(if changed.get() {
                    ".box.changed"
                } else {
                    ".box"
                }))
                .rect,
            );
        };
        frame(&context, 400.0, 0.0, vec![], &mut build);
        changed.set(true);
        frame(&context, 400.0, 1.0, vec![], &mut build);
        frame(&context, 400.0, 1.5, vec![], &mut build);
        assert!(bounds.get().is_finite(), "{percentage}: {:?}", bounds.get());
        assert!(
            (1000.0..=1.0e12).contains(&bounds.get().width()),
            "{percentage}: {:?}",
            bounds.get()
        );
    }
}
