use crate::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaData {
    #[serde(default, skip_serializing_if = "is_false")]
    pub nullable: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub read_only: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub write_only: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub deprecated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocumentation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<Discriminator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// All extensions must be prefixed with `x-`, see
    /// section Specification Extensions on https://swagger.io/specification/
    /// for more information. So you could add a custom field `name` like:
    /// `x-name: value` rather than `name: value`
    /// In code, the `x-` prefix remains as part of the key: `extensions.get("x-name")`
    #[serde(flatten, deserialize_with = "crate::util::deserialize_extensions")]
    pub extensions: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Schema {
    #[serde(flatten)]
    pub data: SchemaData,
    #[serde(flatten)]
    pub kind: SchemaKind,
}

impl std::ops::Deref for Schema {
    type Target = SchemaData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl std::ops::DerefMut for Schema {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum SchemaKind {
    Type(Type),
    OneOf {
        #[serde(rename = "oneOf")]
        one_of: Vec<RefOr<Schema>>,
    },
    AllOf {
        #[serde(rename = "allOf")]
        all_of: Vec<RefOr<Schema>>,
    },
    AnyOf {
        #[serde(rename = "anyOf")]
        any_of: Vec<RefOr<Schema>>,
    },
    Not {
        not: Box<RefOr<Schema>>,
    },
    Any(AnySchema),
}


impl Schema {
    fn new_kind(kind: SchemaKind) -> Self {
        Self { data: SchemaData::default(), kind }
    }

    pub fn new_number() -> Self {
        Self::new_kind(SchemaKind::Type(Type::Number(NumberType::default())))
    }

    pub fn new_integer() -> Self {
        Self::new_kind(SchemaKind::Type(Type::Integer(IntegerType::default())))
    }

    pub fn new_bool() -> Self {
        Self::new_kind(SchemaKind::Type(Type::Boolean {}))
    }

    pub fn new_str_enum(enumeration: Vec<String>) -> Self {
        Self::new_kind(SchemaKind::Type(Type::String(StringType {
            enumeration,
            ..StringType::default()
        })))
    }

    pub fn new_string() -> Self {
        Self::new_kind(SchemaKind::Type(Type::String(StringType::default())))
    }

    /// Create a schemaless object schema
    pub fn new_object() -> Self {
        Self::new_kind(SchemaKind::Type(Type::Object(ObjectType::default())))
    }

    /// Create a Map<String, inner> schema
    pub fn new_map(inner: impl Into<RefOr<Schema>>) -> Self {
        let inner = inner.into().boxed();
        Self::new_kind(SchemaKind::Type(Type::Object(ObjectType {
            additional_properties: Some(AdditionalProperties::Schema(inner)),
            ..ObjectType::default()
        })))
    }

    /// Create a Map<String, Any> schema
    pub fn new_map_any() -> Self {
        Self::new_kind(SchemaKind::Type(Type::Object(ObjectType {
            additional_properties: Some(AdditionalProperties::Any(true)),
            ..ObjectType::default()
        })))
    }

    /// Create an Array<Any> schema
    pub fn new_array_any() -> Self {
        Self::new_kind(SchemaKind::Type(Type::Array(ArrayType::default())))
    }

    /// Create a new array schema with items of the given type
    pub fn new_array(inner: impl Into<RefOr<Schema>>) -> Self {
        let inner = inner.into().boxed();
        Self::new_kind(SchemaKind::Type(Type::Array(ArrayType {
            items: Some(inner),
            ..ArrayType::default()
        })))
    }

    pub fn new_one_of(one_of: Vec<RefOr<Schema>>) -> Self {
        Self::new_kind(SchemaKind::OneOf { one_of })
    }

    pub fn new_all_of(all_of: Vec<RefOr<Schema>>) -> Self {
        Self::new_kind(SchemaKind::AllOf { all_of })
    }

    pub fn new_any_of(any_of: Vec<RefOr<Schema>>) -> Self {
        Self::new_kind(SchemaKind::AnyOf { any_of })
    }

    /// Create an Any schema
    pub fn new_any() -> Self {
        Self {
            data: SchemaData::default(),
            kind: SchemaKind::Any(AnySchema::default()),
        }
    }

    pub fn with_format(mut self, format: &str) -> Self {
        if let SchemaKind::Type(Type::String(s)) = &mut self.kind {
            s.format = serde_json::from_value(Value::String(format.to_string())).unwrap();
        }
        self
    }

    pub fn is_empty(&self) -> bool {
        match &self.kind {
            SchemaKind::Type(Type::Object(o)) => {
                o.properties.is_empty() && o.additional_properties.is_none()
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Type {
    String(StringType),
    Number(NumberType),
    Integer(IntegerType),
    Object(ObjectType),
    Array(ArrayType),
    Boolean {},
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Any(bool),
    Schema(Box<RefOr<Schema>>),
}

/// Catch-all for any combination of properties that doesn't correspond to one
/// of the predefined subsets.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnySchema {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub properties: RefOrMap<Schema>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<AdditionalProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_properties: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_properties: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<RefOr<Schema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_items: Option<bool>,
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enumeration: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub one_of: Vec<RefOr<Schema>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_of: Vec<RefOr<Schema>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_of: Vec<RefOr<Schema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<RefOr<Schema>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StringType {
    #[serde(default, skip_serializing_if = "VariantOrUnknownOrEmpty::is_empty")]
    pub format: VariantOrUnknownOrEmpty<StringFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enumeration: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NumberType {
    #[serde(default, skip_serializing_if = "VariantOrUnknownOrEmpty::is_empty")]
    pub format: VariantOrUnknownOrEmpty<NumberFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub exclusive_minimum: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub exclusive_maximum: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enumeration: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntegerType {
    #[serde(default, skip_serializing_if = "VariantOrUnknownOrEmpty::is_empty")]
    pub format: VariantOrUnknownOrEmpty<IntegerFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<i64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub exclusive_minimum: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub exclusive_maximum: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enumeration: Vec<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObjectType {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub properties: RefOrMap<Schema>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<AdditionalProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_properties: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_properties: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArrayType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<RefOr<Schema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<usize>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub unique_items: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NumberFormat {
    Float,
    Double,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IntegerFormat {
    Int32,
    Int64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StringFormat {
    Date,
    #[serde(rename = "date-time")]
    DateTime,
    Password,
    Byte,
    Binary,
}

impl VariantOrUnknownOrEmpty<StringFormat> {
    pub fn as_str(&self) -> &str {
        match self {
            VariantOrUnknownOrEmpty::Item(StringFormat::Date) => "date",
            VariantOrUnknownOrEmpty::Item(StringFormat::DateTime) => "date-time",
            VariantOrUnknownOrEmpty::Item(StringFormat::Password) => "password",
            VariantOrUnknownOrEmpty::Item(StringFormat::Byte) => "byte",
            VariantOrUnknownOrEmpty::Item(StringFormat::Binary) => "binary",
            VariantOrUnknownOrEmpty::Unknown(s) => s.as_str(),
            VariantOrUnknownOrEmpty::Empty => "",
        }
    }
}


impl Schema {
    pub fn properties(&self) -> &RefOrMap<Schema> {
        self.get_properties().expect("Schema is not an object")
    }

    pub fn get_properties(&self) -> Option<&RefOrMap<Schema>> {
        match &self.kind {
            SchemaKind::Type(Type::Object(o)) => Some(&o.properties),
            SchemaKind::Any(AnySchema { properties, .. }) => Some(properties),
            _ => None,
        }
    }

    pub fn get_properties_mut(&mut self) -> Option<&mut RefOrMap<Schema>> {
        match &mut self.kind {
            SchemaKind::Type(Type::Object(ref mut o)) => Some(&mut o.properties),
            SchemaKind::Type(Type::Array(ref mut a)) => a
                .items
                .as_mut()
                .map(|s| s.as_mut().as_mut().unwrap().properties_mut()),
            SchemaKind::Any(AnySchema { ref mut properties, .. }) => Some(properties),
            _ => None,
        }
    }

    pub fn properties_mut(&mut self) -> &mut RefOrMap<Schema> {
        self.get_properties_mut().expect("Schema is not an object")
    }

    pub fn properties_iter<'a>(&'a self, spec: &'a OpenAPI) -> Box<dyn Iterator<Item=(&'a String, &'a RefOr<Schema>)> + 'a> {
        match &self.kind {
            SchemaKind::Type(Type::Object(o)) => Box::new(o.properties.iter()),
            SchemaKind::Any(AnySchema { properties, .. }) => Box::new(properties.iter()),
            SchemaKind::AllOf { all_of } => Box::new(all_of
                .iter()
                .map(move |schema| schema.resolve(spec).properties_iter(spec))
                .flatten()),
            _ => Box::new(std::iter::empty())
        }
    }

    pub fn is_required(&self, field: &str) -> bool {
        match &self.kind {
            SchemaKind::Type(Type::Object(o)) => o.required.iter().any(|s| s == field),
            SchemaKind::Any(AnySchema { required, .. }) => required.iter().any(|s| s == field),
            _ => true,
        }
    }

    pub fn get_required(&self) -> Option<&Vec<String>> {
        match &self.kind {
            SchemaKind::Type(Type::Object(o)) => Some(&o.required),
            SchemaKind::Any(AnySchema { required, .. }) => Some(required),
            _ => None,
        }
    }

    pub fn required(&self) -> &Vec<String> {
        self.get_required().expect("Schema is not an object")
    }

    pub fn get_required_mut(&mut self) -> Option<&mut Vec<String>> {
        match &mut self.kind {
            SchemaKind::Type(Type::Object(ref mut o)) => Some(&mut o.required),
            SchemaKind::Type(Type::Array(ref mut a)) => a
                .items
                .as_mut()
                .map(|s| s.as_mut().as_mut().unwrap().get_required_mut().unwrap()),
            SchemaKind::Any(AnySchema { ref mut required, .. }) => Some(required),
            _ => None,
        }
    }

    pub fn required_mut(&mut self) -> &mut Vec<String> {
        self.get_required_mut().expect("Schema is not an object")
    }

    pub fn add_required(&mut self, field: &str) {
        if let Some(req) = self.get_required_mut() {
            if req.iter().any(|f| f == field) {
                return;
            }
            req.push(field.to_string());
        }
    }

    pub fn remove_required(&mut self, field: &str) {
        if let Some(req) = self.get_required_mut() {
            req.retain(|f| f != field);
        }
    }

    pub fn is_anonymous_object(&self) -> bool {
        match &self.kind {
            SchemaKind::Type(Type::Object(o)) => o.properties.is_empty(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use serde_json::json;

    use crate::{AnySchema, Schema, SchemaData, SchemaKind};

    #[test]
    fn test_schema_with_extensions() {
        let schema = serde_json::from_str::<Schema>(
            r#"{
                "type": "boolean",
                "x-foo": "bar"
            }"#,
        )
            .unwrap();

        assert_eq!(
            schema.data.extensions.get("x-foo"),
            Some(&json!("bar"))
        );
    }

    #[test]
    fn test_any() {
        let value = json! { {} };
        serde_json::from_value::<AnySchema>(value).unwrap();
    }

    #[test]
    fn test_not() {
        let value = json! {
            {
                "not": {}
            }
        };

        let schema = serde_json::from_value::<Schema>(value).unwrap();
        assert!(matches!(schema.kind, SchemaKind::Not { not: _ }));
    }

    #[test]
    fn test_null() {
        let value = json! {
            {
                "nullable": true,
                "enum": [ null ],
            }
        };

        let schema = serde_json::from_value::<Schema>(value).unwrap();
        assert!(matches!(
            &schema.data,
            SchemaData { nullable: true, .. }
        ));
        assert!(matches!(
            &schema.kind,
            SchemaKind::Any(AnySchema { enumeration, .. }) if enumeration[0] == json!(null)));
    }

    #[test]
    fn test_default_to_object() {
        let s = r##"
required:
  - definition
properties:
  definition:
    type: string
    description: >
      Serialized definition of the version. This should be an OpenAPI 2.x, 3.x or AsyncAPI 2.x file
      serialized as a string, in YAML or JSON.
    example: |
      {asyncapi: "2.0", "info": { "title: … }}
  references:
    type: array
    description: Import external references used by `definition`. It's usually resources not accessible by Bump servers, like local files or internal URLs.
    items:
      $ref: "#/components/schemas/Reference"
"##.trim();
        let s = serde_yaml::from_str::<Schema>(s).unwrap();
        // assert!(matches!(s.schema_kind, SchemaKind::Type(crate::Type::Object(_))), "Schema kind was not expected {:?}", s.schema_kind);
        assert!(matches!(s.kind, SchemaKind::Any(crate::AnySchema{ ref properties, ..}) if properties.len() == 2), "Schema kind was not expected {:?}", s.kind);
    }

    #[test]
    fn test_all_of() {
        let s = r##"
allOf:
  - $ref: "#/components/schemas/DocumentationRequest"
  - $ref: "#/components/schemas/PreviewRequest"
        "##.trim();
        let s = serde_yaml::from_str::<Schema>(s).unwrap();
        match &s.kind {
            SchemaKind::AllOf { all_of } => {
                assert_eq!(all_of.len(), 2);
                assert!(matches!(all_of[0].as_ref_str(), Some("#/components/schemas/DocumentationRequest")));
                assert!(matches!(all_of[1].as_ref_str(), Some("#/components/schemas/PreviewRequest")));
            }
            _ => panic!("Schema kind was not expected {:?}", s.kind)
        }
    }

    #[test]
    fn test_with_format() {
        use crate::variant_or::VariantOrUnknownOrEmpty;
        let s = Schema::new_string().with_format("date-time");
        let SchemaKind::Type(crate::Type::String(s)) = s.kind else { panic!() };
        assert_matches!(s.format, VariantOrUnknownOrEmpty::Item(crate::StringFormat::DateTime));

        let s = Schema::new_string().with_format("uuid");
        let SchemaKind::Type(crate::Type::String(s)) = s.kind else { panic!() };
        assert_matches!(s.format, VariantOrUnknownOrEmpty::Unknown(s) if s == "uuid");
    }
}

