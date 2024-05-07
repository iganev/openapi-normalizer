use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;
use openapiv3::OpenAPI;
use openapiv3::Schema;
use serde_json;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Schema file
    #[arg(short, long)]
    schema: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let path = Path::new(&args.schema).canonicalize()?;

    if !path.exists() || !path.is_file() {
        return Err(anyhow!(format!("Cant read file {:?}", path)));
    }

    let mut data = String::new();
    File::open(path).await?.read_to_string(&mut data).await?;

    let openapi: OpenAPI = serde_json::from_str(&data).expect("Could not deserialize input");
    // println!("{:?}", openapi);

    if let Some(components) = openapi.components.as_ref() {
        for (name, schema) in components.schemas.iter() {
            if let Some(schema) = schema.as_item() {
                let is_complex = is_complex(schema);

                if !is_complex {
                    println!("Key {} => {:?}", name, schema.schema_kind);
                    println!();
                }
            }
        }
    }

    Ok(())
}

pub fn is_complex(schema: &Schema) -> bool {
    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(schema_kind_type) => match schema_kind_type {
            openapiv3::Type::String(string_schema) => !string_schema.enumeration.is_empty(),
            openapiv3::Type::Number(number_schema) => !number_schema.enumeration.is_empty(),
            openapiv3::Type::Integer(int_schema) => !int_schema.enumeration.is_empty(),
            openapiv3::Type::Object(_object_schema) => true,
            openapiv3::Type::Array(array_schema) => array_schema
                .items
                .as_ref()
                .map(|items_schema| {
                    items_schema
                        .as_item()
                        .map(|schema| is_complex(schema))
                        .unwrap_or(false)
                })
                .unwrap_or(false),
            openapiv3::Type::Boolean(_bool_schema) => false,
        },
        openapiv3::SchemaKind::OneOf { one_of } => one_of
            .iter()
            .any(|schema| schema.as_item().map(|s| is_complex(s)).unwrap_or(false)),
        openapiv3::SchemaKind::AllOf { all_of } => all_of
            .iter()
            .any(|schema| schema.as_item().map(|s| is_complex(s)).unwrap_or(false)),
        openapiv3::SchemaKind::AnyOf { any_of } => any_of
            .iter()
            .any(|schema| schema.as_item().map(|s| is_complex(s)).unwrap_or(false)),
        openapiv3::SchemaKind::Not { not } => not.as_item().map(|s| is_complex(s)).unwrap_or(false),
        openapiv3::SchemaKind::Any(schema_kind_any) => {
            if schema_kind_any.items.is_some() || !schema_kind_any.enumeration.is_empty() {
                true
            } else {
                false
            }
        }
    }
}
