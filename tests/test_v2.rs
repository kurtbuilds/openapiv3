#![cfg(feature = "v2")]

use indexmap::IndexMap;
use openapiv3::*;
use std::{println, string::String};

const PETSTORE_EXAMPLE: &str = include_str!("../fixtures/petstore-extended-swagger2-0.json");
const PETSTORE_FORMDATA_EXAMPLE: &str =
    include_str!("../fixtures/petstore-form-data-swagger2-0.json");

#[test]
fn load_swagger_20_and_upgrade() {
    let v2: openapiv3::v2::OpenAPI = serde_json::from_str(PETSTORE_EXAMPLE).unwrap();

    let versioned: openapiv3::VersionedOpenAPI = serde_json::from_str(PETSTORE_EXAMPLE).unwrap();
    assert!(matches!(versioned, openapiv3::VersionedOpenAPI::V2(_)));
    let v3: openapiv3::OpenAPI = versioned.upgrade();

    // schemas
    assert!(v3.openapi.starts_with("3.0"));
    assert!(v2.definitions.as_ref().unwrap().contains_key("Pet"));
    assert!(v2.definitions.as_ref().unwrap().contains_key("NewPet"));
    assert!(v2.definitions.as_ref().unwrap().contains_key("Error"));
    assert!(v3.schemas().contains_key("Pet"));
    assert!(v3.schemas().contains_key("NewPet"));
    assert!(v3.schemas().contains_key("Error"));

    // paths
    assert!(v2.paths.contains_key("/pets"));
    assert!(v2.paths.contains_key("/pets/{id}"));

    assert!(v3.paths.paths.contains_key("/pets"));
    assert!(v3.paths.paths.contains_key("/pets/{id}"));
}

#[test]
fn load_swagger_20_with_form_data_file_and_upgrade() {
    let v2: openapiv3::v2::OpenAPI = serde_json::from_str(PETSTORE_FORMDATA_EXAMPLE).unwrap();

    let versioned: openapiv3::VersionedOpenAPI =
        serde_json::from_str(PETSTORE_FORMDATA_EXAMPLE).unwrap();
    assert!(matches!(versioned, openapiv3::VersionedOpenAPI::V2(_)));
    let v3: openapiv3::OpenAPI = versioned.upgrade();

    // schemas
    assert!(v3.openapi.starts_with("3.0"));
    assert!(v2.definitions.as_ref().unwrap().contains_key("Pet"));
    assert!(v2.definitions.as_ref().unwrap().contains_key("NewPet"));
    assert!(v2.definitions.as_ref().unwrap().contains_key("Error"));
    assert!(v3.schemas().contains_key("Pet"));
    assert!(v3.schemas().contains_key("NewPet"));
    assert!(v3.schemas().contains_key("Error"));

    // paths
    assert!(v2.paths.contains_key("/pets"));

    assert!(v3.paths.paths.contains_key("/pets"));

    v3.paths.paths.iter().for_each(|(p, item)| {
        if p.eq("/pets") {
            if let ReferenceOr::Item(path_item) = item {
                if let Some(operation) = &path_item.post {
                    if let Some(request_body) = &operation.request_body {
                        if let ReferenceOr::Item(body) = request_body {
                            let has = body.content.contains_key("multipart/form-data");
                            let media = body.content.get("multipart/form-data").unwrap();
                            if let Some(refschema) = &media.schema {
                                if let ReferenceOr::Item(schema) = refschema {
                                    let file_field =
                                        schema.properties().unwrap().get("filename").unwrap();

                                    let file_field = &file_field.as_item().unwrap();

                                    // let keys = schema.properties().unwrap().keys();
                                    let s = format!("{:#?}", file_field.schema_kind);

                                    // Type(
                                    //     String(
                                    //         StringType {
                                    //             format: Item(
                                    //                 Binary,
                                    //             ),
                                    //             pattern: None,
                                    //             enumeration: [],
                                    //             min_length: None,
                                    //             max_length: None,
                                    //         },
                                    //     ),
                                    // )
                                    println!("file prop type:\n  {}\n", s);
                                    let okformat = s.contains("Binary");
                                    assert!(okformat);
                                }
                            }
                            assert!(has);
                        }
                    }
                }
            }
        };
    });
}
