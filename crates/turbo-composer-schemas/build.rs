use reqwest::blocking::Client;
use schemars::schema::{RootSchema, Schema, SchemaObject};
use serde_json::Value;
use std::{env, fs, path::PathBuf};
use typify::{TypeSpace, TypeSpaceSettings};

const SEMVER_REGEX: &str = r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)\
(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)\
(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?\
(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$";

fn main() {
    // Fetch schema.json
    let client = Client::new();
    let mut root: RootSchema = client
        .get("https://getcomposer.org/schema.json")
        .send()
        .expect("request failed")
        .json()
        .expect("invalid JSON schema");

    // Patch the minimum-stability enum
    let mut root_obj: SchemaObject = root.schema.clone().into();
    if let Some(obj_props) = &mut root_obj.object {
        if let Some(min_schema) = obj_props.properties.get_mut("minimum-stability") {
            let mut ms_obj = min_schema.clone().into_object();
            if let Some(vals) = &mut ms_obj.enum_values {
                for v in vals.iter_mut() {
                    if let Some(s) = v.as_str() {
                        *v = Value::String(match s {
                            "rc" => "rc_variant".into(),
                            "RC" => "rc_variant_uppercase".into(),
                            _ => s.into(),
                        });
                    }
                }
            }

            *min_schema = Schema::Object(ms_obj);
        }
    }
    root.schema = Schema::Object(root_obj).into();

    if let Some(def) = root.definitions.get_mut("ComposerPackage_version") {
        let mut obj: SchemaObject = def.clone().into_object();
        if let Some(sv) = &mut obj.string {
            sv.pattern = Some(SEMVER_REGEX.to_string());
        }
        *def = Schema::Object(obj);
    }

    // Generate and write Rust types
    let mut ts = TypeSpace::new(TypeSpaceSettings::default().with_struct_builder(true));
    ts.add_root_schema(root).expect("adding schema failed");
    let out = ts.to_stream().to_string();

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR not set");
    fs::write(PathBuf::from(out_dir).join("schemas.rs"), out).expect("writing schemas.rs failed");
}
