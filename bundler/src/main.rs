use tauri_bundler::bundle::{
    bundle_project, BundleBinary, BundleSettings, PackageSettings, PackageType, Settings,
    SettingsBuilder,
};

static OUT_DIR: &str = "target/release/";

fn main() {
    bundle_project(settings()).unwrap();
}

fn settings() -> Settings {
    let package = PackageSettings {
        product_name: "subsAPIxplr".to_string(),
        version: "0.1.0".to_string(),
        description: "Substrate API explorer".to_string(),
        homepage: None,
        authors: Some(vec!["Ales Tsurko <ales.tsurko@gmail.com>".to_string()]),
        default_run: None,
    };

    let mut bundle_set = BundleSettings::default();
    bundle_set.identifier = Some("by.alestsurko.substrate-api-explorer".to_string());
    bundle_set.icon = Some(vec![
        "assets/32x32.png".to_string(),
        "assets/128x128.png".to_string(),
        "assets/128x128@2x.png".to_string(),
    ]);
    bundle_set.resources = Some(vec!["assets".to_string()]);

    use PackageType::*;

    SettingsBuilder::default()
        .package_settings(package)
        .project_out_directory(OUT_DIR)
        .package_types(vec![MacOsBundle, WindowsMsi, Deb])
        .binaries(vec![BundleBinary::new(
            "substrate_api_explorer".to_string(),
            true,
        )
        .set_src_path(Some("../".to_string()))])
        .build()
        .expect("Can not build settings")
}
