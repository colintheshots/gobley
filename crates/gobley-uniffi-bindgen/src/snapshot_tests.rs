/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Snapshot (golden) tests for the generated Kotlin Multiplatform bindings.
//!
//! These tests parse small, self-contained UDL fixtures into a
//! [`ComponentInterface`], run the full Kotlin Multiplatform code generation, and snapshot the
//! resulting source for every target (`common`, `jvm`, `native`, `stub`) plus the generated C
//! header. They act as a fast regression guard for the binding generator independent of the
//! end-to-end Gradle/Kotlin integration tests.

use uniffi_bindgen::{BindingGenerator, Component, GenerationSettings};

use crate::gen_kotlin_multiplatform::{generate_bindings, MultiplatformBindings};
use crate::KotlinBindingGenerator;

/// Render every Kotlin Multiplatform target for a single UDL fixture.
fn render_all_targets(namespace: &str, udl: &str) -> MultiplatformBindings {
    let ci = uniffi_bindgen::ComponentInterface::from_webidl(udl, namespace)
        .expect("failed to parse UDL fixture");

    // Enable Kotlin Multiplatform and request every target so each generated file is exercised.
    let config_toml = r#"
kotlin_multiplatform = true
kotlin_targets = ["jvm", "android", "native", "stub"]
"#;
    let toml_value: toml::Value =
        toml::from_str(config_toml).expect("failed to parse fixture config toml");

    let generator = KotlinBindingGenerator;
    let config = generator
        .new_config(&toml_value)
        .expect("failed to build config");

    let settings = GenerationSettings {
        out_dir: camino::Utf8PathBuf::from("."),
        try_format_code: false,
        cdylib: Some(format!("uniffi_{namespace}")),
    };

    // `update_component_configs` fills in the package name and cdylib name defaults, then
    // `derive_ffi_funcs` populates the FFI function argument/return types. This mirrors the order
    // used by the real `generate_external_bindings` flow in uniffi_bindgen.
    let mut components = vec![Component { ci, config }];
    generator
        .update_component_configs(&settings, &mut components)
        .expect("failed to update component configs");
    components[0]
        .ci
        .derive_ffi_funcs()
        .expect("failed to derive FFI functions");

    let Component { ci, config } = &components[0];
    generate_bindings(config, ci).expect("failed to generate bindings")
}

/// Assert snapshots for every generated target of a fixture.
fn assert_all_target_snapshots(name: &str, bindings: &MultiplatformBindings) {
    insta::assert_snapshot!(format!("{name}.common.kt"), bindings.common);
    insta::assert_snapshot!(
        format!("{name}.jvm.kt"),
        bindings.jvm.as_deref().unwrap_or("")
    );
    insta::assert_snapshot!(
        format!("{name}.native.kt"),
        bindings.native.as_deref().unwrap_or("")
    );
    insta::assert_snapshot!(
        format!("{name}.stub.kt"),
        bindings.stub.as_deref().unwrap_or("")
    );
    insta::assert_snapshot!(
        format!("{name}.header.h"),
        bindings.header.as_deref().unwrap_or("")
    );
}

#[test]
fn snapshot_simple_functions() {
    let udl = r#"
namespace simple_fns {
    string get_string();
    u8 get_u8();
    u16 get_u16();
    u32 add(u32 a, u32 b);
    boolean identity_bool(boolean value);
    bytes echo_bytes(bytes value);
};
"#;
    let bindings = render_all_targets("simple_fns", udl);
    assert_all_target_snapshots("simple_fns", &bindings);
}

#[test]
fn snapshot_records_with_defaults() {
    let udl = r#"
namespace records {
    Config make_config();
};

dictionary Config {
    string name;
    u32 retries = 3;
    boolean verbose = false;
    sequence<string> tags = [];
    string? description = null;
};
"#;
    let bindings = render_all_targets("records", udl);
    assert_all_target_snapshots("records", &bindings);
}

#[test]
fn snapshot_enums() {
    let udl = r#"
namespace enums {
    Shape default_shape();
};

enum Color {
    "Red",
    "Green",
    "Blue",
};

[Enum]
interface Shape {
    Circle(double radius);
    Rectangle(double width, double height);
    Empty();
};
"#;
    let bindings = render_all_targets("enums", udl);
    assert_all_target_snapshots("enums", &bindings);
}

#[test]
fn snapshot_interface_with_methods() {
    // Exercises object handles (FfiType::Handle) for constructors, methods, and return values.
    let udl = r#"
namespace counter {
    Counter make_counter(i32 initial);
};

interface Counter {
    constructor(i32 initial);
    void increment();
    i32 value();
    Counter clone_counter();
};
"#;
    let bindings = render_all_targets("counter", udl);
    assert_all_target_snapshots("counter", &bindings);
}

#[test]
fn snapshot_error_types() {
    let udl = r#"
namespace errors {
    [Throws=ArithmeticError]
    u64 checked_add(u64 a, u64 b);
};

[Error]
enum ArithmeticError {
    "Overflow",
    "DivisionByZero",
};
"#;
    let bindings = render_all_targets("errors", udl);
    assert_all_target_snapshots("errors", &bindings);
}
