//! Builds each tool's input schema from its OpenAPI operation, so tool inputs cannot drift from the API.

use serde_json::{Value, json};

use super::tools::{BodyKind, Kind, ToolDef};

pub type JsonObject = serde_json::Map<String, Value>;

pub const ITEMS_ARG: &str = "items";
pub const CONFIRM_ARG: &str = "confirm";
const REF_PREFIX: &str = "#/components/schemas/";
const MAX_REF_DEPTH: usize = 12;

/// Where the arguments go. Path parameters are read from the path template itself.
#[derive(Debug, Default, Clone)]
pub struct ArgMap {
    pub query: Vec<String>,
    /// The JSON body's fields, which are top-level arguments.
    pub body_fields: Vec<String>,
    /// The JSON body is an array, passed as the `items` argument.
    pub body_array: bool,
}

/// The tool's input schema and argument map. Errors when the operation is missing or its body is unreadable.
pub fn build(doc: &Value, def: &ToolDef) -> Result<(JsonObject, ArgMap), String> {
    let op = &doc["paths"][def.path][def.method];
    if !op.is_object() {
        return Err(format!(
            "{} {} ({}) is not in the OpenAPI document",
            def.method, def.path, def.name
        ));
    }
    let components = &doc["components"]["schemas"];
    let mut props = JsonObject::new();
    let mut required: Vec<Value> = Vec::new();
    let mut args = ArgMap::default();

    for p in op["parameters"].as_array().into_iter().flatten() {
        let (Some(name), Some(place)) = (p["name"].as_str(), p["in"].as_str()) else {
            continue;
        };
        // Header parameters (Idempotency-Key) are not tool arguments.
        if place != "path" && place != "query" {
            continue;
        }
        let mut s = inline(&p["schema"], components, 0);
        if let (Some(o), Some(d)) = (s.as_object_mut(), p["description"].as_str()) {
            o.insert("description".into(), json!(d));
        }
        if place == "path" || p["required"] == json!(true) {
            required.push(json!(name));
        }
        if place == "query" {
            args.query.push(name.into());
        }
        props.insert(name.into(), s);
    }

    match def.body {
        BodyKind::CsvUpload => {
            props.insert("csv".into(), json!({"type": "string",
                "description": "The CSV text. The first row is the header and needs an `email` column. Up to 100,000 rows."}));
            props.insert(
                "consent".into(),
                json!({"type": "boolean",
                "description": "Must be true: these people agreed to get your emails."}),
            );
            props.insert(
                "tag_id".into(),
                json!({"type": "string", "format": "uuid",
                "description": "Optional tag id (see list_tags) added to every imported contact."}),
            );
            required.extend([json!("csv"), json!("consent")]);
        }
        BodyKind::Json => {
            let raw = &op["requestBody"]["content"]["application/json"]["schema"];
            if !raw.is_null() {
                let body = inline(raw, components, 0);
                if body["type"] == json!("array") {
                    props.insert(ITEMS_ARG.into(), body);
                    required.push(json!(ITEMS_ARG));
                    args.body_array = true;
                } else {
                    let fields = body["properties"].as_object().ok_or_else(|| {
                        format!("{}: the request body has no properties", def.name)
                    })?;
                    for (k, v) in fields {
                        if props.contains_key(k) {
                            return Err(format!(
                                "{}: body field `{k}` clashes with a parameter",
                                def.name
                            ));
                        }
                        props.insert(k.clone(), v.clone());
                        args.body_fields.push(k.clone());
                    }
                    required.extend(body["required"].as_array().into_iter().flatten().cloned());
                }
            }
        }
    }

    if def.kind == Kind::Destructive {
        props.insert(CONFIRM_ARG.into(), json!({"type": "boolean",
            "description": "Set to true to do it. Without it nothing changes and you get a preview of what would happen."}));
    }

    let mut schema = JsonObject::new();
    schema.insert("type".into(), json!("object"));
    schema.insert("properties".into(), Value::Object(props));
    schema.insert("required".into(), Value::Array(required));
    Ok((schema, args))
}

/// Replaces `#/components/schemas/X` references with the schema itself, so a tool schema stands alone.
fn inline(v: &Value, components: &Value, depth: usize) -> Value {
    match v {
        Value::Object(o) => {
            if let Some(name) = o
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| r.strip_prefix(REF_PREFIX))
            {
                // shortcut: a type deeper than this becomes "any"; no Mailrust type nests this deep today.
                if depth >= MAX_REF_DEPTH {
                    return json!({});
                }
                return inline(&components[name], components, depth + 1);
            }
            Value::Object(
                o.iter()
                    .map(|(k, v)| (k.clone(), inline(v, components, depth)))
                    .collect(),
            )
        }
        Value::Array(a) => Value::Array(a.iter().map(|v| inline(v, components, depth)).collect()),
        _ => v.clone(),
    }
}
