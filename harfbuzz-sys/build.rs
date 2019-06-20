#[cfg(feature = "build-native-harfbuzz")]
extern crate cmake;
#[cfg(feature = "build-native-harfbuzz")]
extern crate pkg_config;

fn emit_special_c_env_vars(libs: Vec<pkg_config::Library>) {
    {
        let mut all_include_dirs: Vec<std::path::PathBuf> = Vec::new();
        for lib in libs.iter() {
            all_include_dirs.extend(lib.include_paths.clone());
        }
        let ser_include_dirs = serde_json::to_string(&all_include_dirs).unwrap();
        println!("cargo:include_dirs={}", base64::encode(&ser_include_dirs));
    }

    {
        let mut libdir_flags = String::new();
        for lib in libs.iter() {
            for libdir in lib.link_paths.iter() {
                match libdir.to_str() {
                    Some(path) => libdir_flags.push_str(&format!("-L{} ", path)),
                    None => (),
                }
            }
        }
        println!("cargo:libdir_flags={}", libdir_flags);
    }

    {
        let mut link_flags = String::new();
        for lib in libs.iter() {
            for l in lib.libs.iter() {
                link_flags.push_str(&format!("-l{} ", l))
            }
        }
        println!("cargo:link_flags={}", link_flags);
    }
}

#[cfg(feature = "build-native-harfbuzz")]
fn main() {
    use std::env;
    use std::path::PathBuf;
    use std::process::Command;

    let target = env::var("TARGET").unwrap();

    println!("cargo:rerun-if-env-changed=HARFBUZZ_SYS_NO_PKG_CONFIG");
    if target.contains("wasm32") || env::var_os("HARFBUZZ_SYS_NO_PKG_CONFIG").is_none() {
        let mut config = pkg_config::Config::new();
        config.statik(true).cargo_metadata(true);

        if let Ok(libhb) = config.atleast_version("1.4").probe("harfbuzz") {
            if let Ok(libhb_icu) = config.atleast_version("1.4").probe("harfbuzz-icu") {
                emit_special_c_env_vars(vec![libhb, libhb_icu]);
                return;
            }
        }
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // On Windows, HarfBuzz configures atomics directly; otherwise,
    // it needs assistance from configure to do so.  Just use the makefile
    // build for now elsewhere.
    if target.contains("windows") || target.contains("wasm32") {
        let mut cfg = cmake::Config::new("harfbuzz");
        if target.contains("wasm") {
            // When building on macOS for wasm32, make sure we aren't picking up
            // CoreText.
            cfg.define("HB_HAVE_CORETEXT", "OFF");
            if target == "wasm32-unknown-unknown" {
                // Switch to the correct target triple for the underlying toolchain.
                cfg.target("wasm32-unknown-none");
            }
        }
        let dst = cfg.build();
        println!("cargo:rustc-link-search=native={}/lib", dst.display());
        println!("cargo:rustc-link-lib=static=harfbuzz");
        if target.contains("gnu") {
            println!("cargo:rustc-link-lib=stdc++");
        }
    } else {
        assert!(Command::new("make")
            .env("MAKEFLAGS", env::var("CARGO_MAKEFLAGS").unwrap_or_default())
            .args(&["-R", "-f", "makefile.cargo"])
            .status()
            .unwrap()
            .success());

        println!(
            "cargo:rustc-link-search=native={}",
            out_dir.join("lib").display()
        );
        println!("cargo:rustc-link-lib=static=harfbuzz");
        println!("cargo:rustc-link-lib=static=harfbuzz-icu");
        println!("cargo:libdir_flags=-L{}/lib", out_dir.display());
        println!("cargo:link_flags=-l:libharfbuzz.a -l:libharfbuzz-icu.a");
    }

    // DEP_HARFBUZZ_INCLUDE has the path of the vendored harfbuzz.
    println!(
        "cargo:include={}",
        out_dir.join("include").join("harfbuzz").display()
    );
}

#[cfg(not(feature = "build-native-harfbuzz"))]
fn main() {}
