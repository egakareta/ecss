// Conversion matrices and extended transfer curves follow CSS Color 4 §19.
#[derive(Clone, Copy)]
pub(super) enum ColorSpace {
    Srgb,
    SrgbLinear,
    DisplayP3,
    DisplayP3Linear,
    A98Rgb,
    ProPhotoRgb,
    Rec2020,
    XyzD50,
    XyzD65,
}

impl ColorSpace {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("srgb") {
            Some(Self::Srgb)
        } else if name.eq_ignore_ascii_case("srgb-linear") {
            Some(Self::SrgbLinear)
        } else if name.eq_ignore_ascii_case("display-p3") {
            Some(Self::DisplayP3)
        } else if name.eq_ignore_ascii_case("display-p3-linear") {
            Some(Self::DisplayP3Linear)
        } else if name.eq_ignore_ascii_case("a98-rgb") {
            Some(Self::A98Rgb)
        } else if name.eq_ignore_ascii_case("prophoto-rgb") {
            Some(Self::ProPhotoRgb)
        } else if name.eq_ignore_ascii_case("rec2020") {
            Some(Self::Rec2020)
        } else if name.eq_ignore_ascii_case("xyz-d50") {
            Some(Self::XyzD50)
        } else if name.eq_ignore_ascii_case("xyz-d65") || name.eq_ignore_ascii_case("xyz") {
            Some(Self::XyzD65)
        } else {
            None
        }
    }
}

pub(super) fn predefined_to_srgb(space: ColorSpace, components: [f64; 3]) -> [f64; 3] {
    if matches!(space, ColorSpace::Srgb) {
        return components;
    }

    let (linear_rgb_or_xyz, d50) = match space {
        ColorSpace::Srgb => (linear_srgb(components), false),
        ColorSpace::SrgbLinear => (components, false),
        ColorSpace::DisplayP3 => (linear_srgb(components), false),
        ColorSpace::DisplayP3Linear => (components, false),
        ColorSpace::A98Rgb => (components.map(a98_to_linear), false),
        ColorSpace::ProPhotoRgb => (components.map(prophoto_to_linear), true),
        ColorSpace::Rec2020 => (components.map(rec2020_to_linear), false),
        ColorSpace::XyzD50 => (components, true),
        ColorSpace::XyzD65 => (components, false),
    };

    let xyz_d65 = match space {
        ColorSpace::XyzD50 | ColorSpace::XyzD65 => {
            if d50 {
                d50_to_d65(linear_rgb_or_xyz)
            } else {
                linear_rgb_or_xyz
            }
        }
        ColorSpace::Srgb | ColorSpace::SrgbLinear => matrix_vector(
            [
                [506752.0 / 1228815.0, 87881.0 / 245763.0, 12673.0 / 70218.0],
                [87098.0 / 409605.0, 175762.0 / 245763.0, 12673.0 / 175545.0],
                [7918.0 / 409605.0, 87881.0 / 737289.0, 1001167.0 / 1053270.0],
            ],
            linear_rgb_or_xyz,
        ),
        ColorSpace::DisplayP3 | ColorSpace::DisplayP3Linear => matrix_vector(
            [
                [
                    608311.0 / 1250200.0,
                    189793.0 / 714400.0,
                    198249.0 / 1000160.0,
                ],
                [
                    35783.0 / 156275.0,
                    247089.0 / 357200.0,
                    198249.0 / 2500400.0,
                ],
                [0.0, 32229.0 / 714400.0, 5220557.0 / 5000800.0],
            ],
            linear_rgb_or_xyz,
        ),
        ColorSpace::A98Rgb => matrix_vector(
            [
                [
                    573536.0 / 994567.0,
                    263643.0 / 1420810.0,
                    187206.0 / 994567.0,
                ],
                [
                    591459.0 / 1989134.0,
                    6239551.0 / 9945670.0,
                    374412.0 / 4972835.0,
                ],
                [
                    53769.0 / 1989134.0,
                    351524.0 / 4972835.0,
                    4929758.0 / 4972835.0,
                ],
            ],
            linear_rgb_or_xyz,
        ),
        ColorSpace::ProPhotoRgb => d50_to_d65(matrix_vector(
            [
                [0.7977666449006423, 0.13518129740053308, 0.0313477341283922],
                [0.2880748288194013, 0.711835234241873, 0.00008993693872564],
                [0.0, 0.0, 0.8251046025104602],
            ],
            linear_rgb_or_xyz,
        )),
        ColorSpace::Rec2020 => matrix_vector(
            [
                [
                    63426534.0 / 99577255.0,
                    20160776.0 / 139408157.0,
                    47086771.0 / 278816314.0,
                ],
                [
                    26158966.0 / 99577255.0,
                    472592308.0 / 697040785.0,
                    8267143.0 / 139408157.0,
                ],
                [0.0, 19567812.0 / 697040785.0, 295819943.0 / 278816314.0],
            ],
            linear_rgb_or_xyz,
        ),
    };

    xyz_d65_to_srgb(xyz_d65)
}

pub(super) fn lab_to_srgb(lab: [f64; 3]) -> [f64; 3] {
    let [lightness, a, b] = lab;
    let kappa = 24389.0 / 27.0;
    let epsilon = 216.0 / 24389.0;
    let lightness_116 = lightness / 116.0;
    let a_500 = a / 500.0;
    let b_200 = b / 200.0;
    let fy = lightness_116 + 16.0 / 116.0;
    let fx = fy + a_500;
    let fz = fy - b_200;
    let xyz_d50 = [
        if fx * fx * fx > epsilon {
            fx * fx * fx
        } else {
            (lightness_116 + a_500) * (116.0 / kappa)
        } * (0.3457 / 0.3585),
        if lightness > kappa * epsilon {
            ((lightness + 16.0) / 116.0).powi(3)
        } else {
            lightness / kappa
        },
        if fz * fz * fz > epsilon {
            fz * fz * fz
        } else {
            (lightness_116 - b_200) * (116.0 / kappa)
        } * ((1.0 - 0.3457 - 0.3585) / 0.3585),
    ];
    xyz_d50_to_srgb(xyz_d50)
}

pub(super) fn oklab_to_srgb(oklab: [f64; 3]) -> [f64; 3] {
    let [lightness, a, b] = oklab;
    let l = lightness + 0.3963377773761749 * a + 0.2158037573099136 * b;
    let m = lightness - 0.1055613458156586 * a - 0.0638541728258133 * b;
    let s = lightness - 0.0894841775298119 * a - 1.2914855480194092 * b;
    let xyz_d65 = matrix_vector(
        [
            [1.2268798758459243, -0.5578149944602171, 0.2813910456659647],
            [-0.0405757452148008, 1.112286803280317, -0.0717110580655164],
            [-0.0763729366746601, -0.4214933324022432, 1.5869240198367816],
        ],
        [l.powi(3), m.powi(3), s.powi(3)],
    );
    xyz_d65_to_srgb(xyz_d65)
}

fn xyz_d50_to_srgb(xyz_d50: [f64; 3]) -> [f64; 3] {
    xyz_d65_to_srgb(d50_to_d65(xyz_d50))
}

fn d50_to_d65(xyz: [f64; 3]) -> [f64; 3] {
    matrix_vector(
        [
            [0.955473421488075, -0.02309845494876471, 0.06325924320057072],
            [
                -0.0283697093338637,
                1.0099953980813041,
                0.021041441191917323,
            ],
            [
                0.012314014864481998,
                -0.020507649298898964,
                1.330365926242124,
            ],
        ],
        xyz,
    )
}

fn xyz_d65_to_srgb(xyz: [f64; 3]) -> [f64; 3] {
    let linear = matrix_vector(
        [
            [12831.0 / 3959.0, -329.0 / 214.0, -1974.0 / 3959.0],
            [
                -851781.0 / 878810.0,
                1648619.0 / 878810.0,
                36519.0 / 878810.0,
            ],
            [705.0 / 12673.0, -2585.0 / 12673.0, 705.0 / 667.0],
        ],
        xyz,
    );
    linear.map(srgb_gamma_encode)
}

fn linear_srgb(encoded: [f64; 3]) -> [f64; 3] {
    encoded.map(srgb_gamma_decode)
}

fn srgb_gamma_decode(value: f64) -> f64 {
    let absolute = value.abs();
    if absolute <= 0.04045 {
        value / 12.92
    } else {
        value.signum() * ((absolute + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb_gamma_encode(value: f64) -> f64 {
    let absolute = value.abs();
    if absolute > 0.0031308 {
        value.signum() * (1.055 * absolute.powf(1.0 / 2.4) - 0.055)
    } else {
        12.92 * value
    }
}

fn a98_to_linear(value: f64) -> f64 {
    value.signum() * value.abs().powf(563.0 / 256.0)
}

fn prophoto_to_linear(value: f64) -> f64 {
    if value.abs() <= 16.0 / 512.0 {
        value / 16.0
    } else {
        value.signum() * value.abs().powf(1.8)
    }
}

fn rec2020_to_linear(value: f64) -> f64 {
    value.signum() * value.abs().powf(2.4)
}

fn matrix_vector(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    matrix.map(|row| row[0] * vector[0] + row[1] * vector[1] + row[2] * vector[2])
}
