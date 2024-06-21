use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;
use openapiv3::OpenAPI;
use openapiv3::ReferenceOr;
use openapiv3::Schema;
use openapiv3::StatusCode;
use serde_json;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub const COMPONENT_SCHEMA: &str = "schemas";
pub const COMPONENT_PARAM: &str = "parameters";
pub const COMPONENT_RESPONSE: &str = "responses";

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

    let mut complex_component_params = HashMap::new();
    let mut simple_component_params = HashMap::new();
    let mut referenced_component_params: Vec<String> = Vec::new();
    let mut redundant_simple_component_params: Vec<String> = Vec::new();

    let mut complex_component_schemas = HashMap::new();
    let mut simple_component_schemas = HashMap::new();
    let mut referenced_component_schemas: Vec<String> = Vec::new();
    let mut redundant_simple_component_schemas: Vec<String> = Vec::new();

    let mut complex_component_responses = HashMap::new();
    let mut simple_component_responses = HashMap::new();
    let mut referenced_component_responses: Vec<String> = Vec::new();
    let mut redundant_simple_component_responses: Vec<String> = Vec::new();

    println!("Collecting schema information");

    if let Some(components) = openapi.components.as_ref() {
        for (name, schema) in components.schemas.iter() {
            match schema {
                ReferenceOr::Reference { reference } => {
                    // reference
                    println!(
                        "Thats weird. Found schema reference {} => {}",
                        name, reference
                    );
                }
                ReferenceOr::Item(schema) => {
                    if is_complex(schema) {
                        complex_component_schemas.insert(name.clone(), schema.clone());
                    } else {
                        simple_component_schemas.insert(name.clone(), schema.clone());
                    }
                }
            }
        }

        for (name, param) in components.parameters.iter() {
            match param {
                ReferenceOr::Reference { reference } => {
                    // reference
                    println!(
                        "Thats weird. Found param reference {} => {}",
                        name, reference
                    );
                }
                ReferenceOr::Item(param) => match &param.parameter_data_ref().format {
                    openapiv3::ParameterSchemaOrContent::Schema(schema) => {
                        if let Some(schema) = schema.as_item() {
                            if is_complex(schema) {
                                complex_component_params.insert(name.clone(), schema.clone());
                            } else {
                                simple_component_params.insert(name.clone(), schema.clone());
                            }
                        } else if let ReferenceOr::Reference { reference } = schema {
                            println!("Found param reference {} => {}", name, reference);
                        }
                    }
                    openapiv3::ParameterSchemaOrContent::Content(content) => {
                        for (_content_key, content_media) in content.iter() {
                            if let Some(schema) = &content_media.schema {
                                match schema {
                                    ReferenceOr::Reference { reference } => {
                                        println!("Found param reference {} => {}", name, reference);
                                    }
                                    ReferenceOr::Item(schema) => {
                                        if is_complex(schema) {
                                            complex_component_params
                                                .insert(name.clone(), schema.clone());
                                        } else {
                                            simple_component_params
                                                .insert(name.clone(), schema.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }

        for (name, response) in components.responses.iter() {
            match response {
                ReferenceOr::Reference { reference } => {
                    println!(
                        "Thats weird. Found response reference {} => {}",
                        name, reference
                    );
                }
                ReferenceOr::Item(response) => {
                    for (header_name, header) in response.headers.iter() {
                        match header {
                            ReferenceOr::Reference { reference } => {
                                println!(
                                    "Thats weird. Found response header reference {} => {}",
                                    name, reference
                                );
                            }
                            ReferenceOr::Item(header) => match &header.format {
                                openapiv3::ParameterSchemaOrContent::Schema(schema) => match schema
                                {
                                    ReferenceOr::Reference { reference } => {
                                        println!(
                                            "Thats weird. Found response header schema reference {} => {}",
                                            name, reference
                                        );
                                    }
                                    ReferenceOr::Item(schema) => {
                                        if is_complex(schema) {
                                            complex_component_responses.insert(
                                                format!("{}/header/{}", name.clone(), header_name),
                                                schema.clone(),
                                            );
                                        } else {
                                            simple_component_responses.insert(
                                                format!("{}/header/{}", name.clone(), header_name),
                                                schema.clone(),
                                            );
                                        }
                                    }
                                },
                                openapiv3::ParameterSchemaOrContent::Content(content) => {
                                    for (_content_key, content_media) in content.iter() {
                                        if let Some(schema) = &content_media.schema {
                                            match schema {
                                                ReferenceOr::Reference { reference } => {
                                                    println!(
                                                        "Thats weird. Found response header reference {} => {}",
                                                        name, reference
                                                    );
                                                }
                                                ReferenceOr::Item(schema) => {
                                                    if is_complex(schema) {
                                                        complex_component_responses.insert(
                                                            format!(
                                                                "{}/header/{}",
                                                                name.clone(),
                                                                header_name
                                                            ),
                                                            schema.clone(),
                                                        );
                                                    } else {
                                                        simple_component_responses.insert(
                                                            format!(
                                                                "{}/header/{}",
                                                                name.clone(),
                                                                header_name
                                                            ),
                                                            schema.clone(),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                        }
                    }

                    for (content_key, content_media) in response.content.iter() {
                        if let Some(schema) = &content_media.schema {
                            match schema {
                                ReferenceOr::Reference { reference } => {
                                    println!(
                                        "Thats weird. Found response content reference {} => {:?}",
                                        name, reference
                                    );
                                }
                                ReferenceOr::Item(schema) => {
                                    if is_complex(schema) {
                                        complex_component_responses.insert(
                                            format!("{}/content/{}", name.clone(), content_key),
                                            schema.clone(),
                                        );
                                    } else {
                                        simple_component_responses.insert(
                                            format!("{}/content/{}", name.clone(), content_key),
                                            schema.clone(),
                                        );
                                    }
                                }
                            }
                        }
                    }

                    for (_link_key, link) in response.links.iter() {
                        match link {
                            ReferenceOr::Reference { reference } => {
                                println!(
                                    "Thats weird. Found response link reference {} => {:?}",
                                    name, reference
                                );
                            }
                            ReferenceOr::Item(_link) => {}
                        }
                    }
                }
            }
        }
    }

    println!();

    println!("Parsing paths information");

    for (name, path) in openapi.paths.iter() {
        println!("Scanning path {}", name);
        if let Some(path) = path.as_item() {
            for (_op_name, operation) in path.iter() {
                for param in operation.parameters.iter() {
                    match param {
                        ReferenceOr::Reference { reference } => {
                            let ref_data = parse_reference(reference);

                            if ref_data.1 == COMPONENT_PARAM {
                                referenced_component_params.push(ref_data.0.to_string());
                            } else if ref_data.1 == COMPONENT_SCHEMA {
                                referenced_component_schemas.push(ref_data.0.to_string());
                            } else {
                                // unhandled component type
                            }

                            println!("Param reference name {} of type {}", ref_data.0, ref_data.1);
                        }
                        ReferenceOr::Item(param) => {
                            match &param.parameter_data_ref().format {
                                openapiv3::ParameterSchemaOrContent::Schema(schema) => {
                                    match schema {
                                        ReferenceOr::Reference { reference } => {
                                            // count references to find reduntant component schemas

                                            let ref_data = parse_reference(reference);

                                            if ref_data.1 == COMPONENT_PARAM {
                                                referenced_component_params
                                                    .push(ref_data.0.to_string());
                                            } else if ref_data.1 == COMPONENT_SCHEMA {
                                                referenced_component_schemas
                                                    .push(ref_data.0.to_string());
                                            } else {
                                                // unhandled component type
                                            }

                                            println!(
                                                "Param {} reference name {} of type {}",
                                                param.parameter_data_ref().name,
                                                ref_data.0,
                                                ref_data.1
                                            );
                                        }
                                        ReferenceOr::Item(schema) => {
                                            if is_complex(schema) {
                                                // this should be a reference, ideally, but is an inline schema
                                                println!(
                                                    "Param schema is complex for {}",
                                                    param.parameter_data_ref().name
                                                );
                                            } else {
                                                // this is a simple type, not necessarily needs to be a schema, only if it repeats
                                                println!(
                                                    "Param schema is simple for {}",
                                                    param.parameter_data_ref().name
                                                );
                                            }
                                        }
                                    }
                                }
                                openapiv3::ParameterSchemaOrContent::Content(content) => {
                                    //not entirely sure yet what that is
                                    for (_content_key, content_media) in content.iter() {
                                        if let Some(schema) = &content_media.schema {
                                            match schema {
                                                ReferenceOr::Reference { reference } => {
                                                    let ref_data = parse_reference(reference);

                                                    if ref_data.1 == COMPONENT_PARAM {
                                                        referenced_component_params
                                                            .push(ref_data.0.to_string());
                                                    } else if ref_data.1 == COMPONENT_SCHEMA {
                                                        referenced_component_schemas
                                                            .push(ref_data.0.to_string());
                                                    } else {
                                                        // unhandled component type
                                                    }

                                                    println!(
                                                        "Param reference name {} of type {}",
                                                        ref_data.0, ref_data.1
                                                    );
                                                }
                                                ReferenceOr::Item(schema) => {
                                                    if is_complex(schema) {
                                                        println!(
                                                            "Param schema is complex for {}",
                                                            param.parameter_data_ref().name
                                                        );
                                                    } else {
                                                        println!(
                                                            "Param schema is simple for {}",
                                                            param.parameter_data_ref().name
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                for (resp_code, resp_obj) in operation.responses.responses.iter().chain(
                    operation
                        .responses
                        .default
                        .iter()
                        .map(|def_resp| (&StatusCode::Code(200), def_resp)),
                ) {
                    match resp_obj {
                        ReferenceOr::Reference { reference } => {
                            // the whole response object is a reference
                            let ref_data = parse_reference(reference);

                            if ref_data.1 == COMPONENT_PARAM {
                                referenced_component_params.push(ref_data.0.to_string());
                            } else if ref_data.1 == COMPONENT_SCHEMA {
                                referenced_component_schemas.push(ref_data.0.to_string());
                            } else {
                                // unhandled component type
                            }

                            println!(
                                "Response reference name {} of type {}",
                                ref_data.0, ref_data.1
                            );
                        }
                        ReferenceOr::Item(resp) => {
                            // the response object is an inline schema
                            for (_content_key, content_media) in resp.content.iter() {
                                if let Some(schema) = &content_media.schema {
                                    match schema {
                                        ReferenceOr::Reference { reference } => {
                                            let ref_data = parse_reference(reference);

                                            if ref_data.1 == COMPONENT_PARAM {
                                                referenced_component_params
                                                    .push(ref_data.0.to_string());
                                            } else if ref_data.1 == COMPONENT_SCHEMA {
                                                referenced_component_schemas
                                                    .push(ref_data.0.to_string());
                                            } else {
                                                // unhandled component type
                                            }

                                            println!(
                                                "Response reference name {} of type {}",
                                                ref_data.0, ref_data.1
                                            );
                                        }
                                        ReferenceOr::Item(schema) => {
                                            if is_complex(schema) {
                                                println!(
                                                    "Response schema is complex for {}",
                                                    resp_code
                                                );
                                            } else {
                                                println!(
                                                    "Response schema is simple for {}",
                                                    resp_code
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                println!();
            }
        }
    }

    println!();

    println!("Report");

    for (param_name, _param_schema) in complex_component_params
        .iter()
        .chain(simple_component_params.iter())
    {
        if !referenced_component_params.contains(param_name) {
            println!("Param {} is never used", param_name);
        }
    }

    for (schema_name, _schema) in complex_component_schemas
        .iter()
        .chain(simple_component_schemas.iter())
    {
        if !referenced_component_schemas.contains(schema_name) {
            println!("Schema {} is never used", schema_name);
        }
    }

    Ok(())
}

pub fn parse_reference(reference: &str) -> (&str, &str) {
    let mut tokens = reference.split('/').rev();

    (
        tokens.next().unwrap_or_default(),
        tokens.next().unwrap_or_default(),
    )
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
