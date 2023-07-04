use std::str::FromStr;

use crate::{OpenAPI, Parameter, RequestBody, Response, Schema};
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

/// A structured enum of an OpenAPI reference.
/// e.g. #/components/schemas/Account or #/components/schemas/Account/properties/name
pub enum SchemaReference {
    Schema { schema: String },
    Property { schema: String, property: String },
}

impl FromStr for SchemaReference {
    type Err = anyhow::Error;

    fn from_str(reference: &str) -> Result<Self> {
        let components = reference.split('/').collect::<Vec<_>>();
        match components.as_slice() {
            ["#", "components", "schemas", schema] => Ok(Self::Schema {
                schema: (*schema).to_owned(),
            }),
            ["#", "components", "schemas", schema, "properties", property] => Ok(Self::Property {
                schema: (*schema).to_owned(),
                property: (*property).to_owned(),
            }),
            _ => bail!("malformed reference; {reference} cannot be parsed as SchemaReference"),
        }
    }
}

impl std::fmt::Display for SchemaReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaReference::Schema { schema } => write!(f, "#/components/schemas/{}", schema),
            SchemaReference::Property { schema, property } => {
                write!(f, "#/components/schemas/{}/properties/{}", schema, property)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum ReferenceOr<T> {
    Reference {
        #[serde(rename = "$ref")]
        reference: String,
    },
    Item(T),
}

impl<T> ReferenceOr<T> {
    pub fn ref_(r: &str) -> Self {
        ReferenceOr::Reference {
            reference: r.to_owned(),
        }
    }
    pub fn item(item: T) -> Self {
        ReferenceOr::Item(item)
    }
    pub fn schema_ref(r: &str) -> Self {
        ReferenceOr::Reference {
            reference: format!("#/components/schemas/{}", r),
        }
    }

    pub fn boxed(self) -> ReferenceOr<Box<T>> {
        match self {
            ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference },
            ReferenceOr::Item(i) => ReferenceOr::Item(Box::new(i)),
        }
    }

    pub fn boxed_item(item: T) -> ReferenceOr<Box<T>> {
        ReferenceOr::Item(Box::new(item))
    }

    /// Converts this [ReferenceOr] to the item inside, if it exists.
    ///
    /// The return value will be [Option::Some] if this was a [ReferenceOr::Item] or [Option::None] if this was a [ReferenceOr::Reference].
    ///
    /// # Examples
    ///
    /// ```
    /// # use openapiv3::ReferenceOr;
    ///
    /// let i = ReferenceOr::Item(1);
    /// assert_eq!(i.into_item(), Some(1));
    ///
    /// let j: ReferenceOr<u8> = ReferenceOr::Reference { reference: String::new() };
    /// assert_eq!(j.into_item(), None);
    /// ```
    pub fn into_item(self) -> Option<T> {
        match self {
            ReferenceOr::Reference { .. } => None,
            ReferenceOr::Item(i) => Some(i),
        }
    }

    /// Returns a reference to to the item inside this [ReferenceOr], if it exists.
    ///
    /// The return value will be [Option::Some] if this was a [ReferenceOr::Item] or [Option::None] if this was a [ReferenceOr::Reference].
    ///
    /// # Examples
    ///
    /// ```
    /// # use openapiv3::ReferenceOr;
    ///
    /// let i = ReferenceOr::Item(1);
    /// assert_eq!(i.as_item(), Some(&1));
    ///
    /// let j: ReferenceOr<u8> = ReferenceOr::Reference { reference: String::new() };
    /// assert_eq!(j.as_item(), None);
    /// ```
    pub fn as_item(&self) -> Option<&T> {
        match self {
            ReferenceOr::Reference { .. } => None,
            ReferenceOr::Item(i) => Some(i),
        }
    }

    pub fn as_ref_str(&self) -> Option<&str> {
        match self {
            ReferenceOr::Reference { reference } => Some(reference),
            ReferenceOr::Item(_) => None,
        }
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            ReferenceOr::Reference { .. } => None,
            ReferenceOr::Item(i) => Some(i),
        }
    }
}

impl<T: 'static> ReferenceOr<T> {
    pub fn as_ref(&self) -> ReferenceOr<&T> {
        match self {
            ReferenceOr::Reference { reference } => ReferenceOr::Reference {
                reference: reference.clone(),
            },
            ReferenceOr::Item(i) => ReferenceOr::Item(i),
        }
    }
}

impl ReferenceOr<Schema> {
    pub fn is_empty(&self) -> bool {
        self.as_item().map(|s| s.is_empty()).unwrap_or(false)
    }
}

impl ReferenceOr<Box<Schema>> {
    pub fn unbox(&self) -> ReferenceOr<Schema> {
        match self {
            ReferenceOr::Reference { reference } => ReferenceOr::Reference {
                reference: reference.clone(),
            },
            ReferenceOr::Item(boxed) => ReferenceOr::Item(*boxed.clone()),
        }
    }
}

fn parse_reference<'a>(reference: &'a str, group: &str) -> Result<&'a str> {
    reference
        .rsplit_once('/')
        .filter(|(head, _name)| head.strip_prefix("#/components/") == Some(group))
        .map(|(_head, name)| name)
        .ok_or_else(|| anyhow!("invalid {} reference: {}", group, reference))
}

fn get_response_name(reference: &str) -> Result<&str> {
    parse_reference(reference, "responses")
}

fn get_request_body_name(reference: &str) -> Result<&str> {
    parse_reference(reference, "requestBodies")
}

fn get_parameter_name(reference: &str) -> Result<&str> {
    parse_reference(reference, "parameters")
}

impl<T: Default> Default for ReferenceOr<T> {
    fn default() -> Self {
        ReferenceOr::Item(T::default())
    }
}

/// Abstract over types which can potentially resolve a contained type, given an `OpenAPI` instance.
pub trait Resolve {
    type Output;

    fn resolve<'a>(&'a self, spec: &'a OpenAPI) -> Result<&'a Self::Output>;
}

impl Resolve for ReferenceOr<Schema> {
    type Output = Schema;

    fn resolve<'a>(&'a self, spec: &'a OpenAPI) -> Result<&'a Self::Output> {
        let reference = match self {
            ReferenceOr::Item(item) => return Ok(item),
            ReferenceOr::Reference { reference } => reference,
        };
        let reference = SchemaReference::from_str(reference)?;
        let get_schema = |schema: &str| -> Result<&Schema> {
            let schema_ref = spec
                .schemas()
                .get(schema)
                .ok_or_else(|| anyhow!("{reference} not found in OpenAPI spec"))?;
            schema_ref.as_item().ok_or_else(|| {
                let ref_ = schema_ref
                    .as_ref_str()
                    .expect("schema_ref was not item so must be ref");
                anyhow!("reference {reference} refers to {ref_}").context(
                    "references must refer directly to the definition; chains are not permitted",
                )
            })
        };
        match &reference {
            SchemaReference::Schema { schema } => get_schema(schema),
            SchemaReference::Property {
                schema: schema_name,
                property,
            } => {
                let schema = get_schema(schema_name)?;
                schema
                    .properties()
                    .ok_or_else(|| anyhow!("tried to resolve reference {reference}, but {schema_name} is not an object with properties"))?
                    .get(property).ok_or_else(|| anyhow!("schema {schema_name} lacks property {property}"))?
                    .resolve(spec)
            }
        }
    }
}

impl Resolve for ReferenceOr<Parameter> {
    type Output = Parameter;

    fn resolve<'a>(&'a self, spec: &'a OpenAPI) -> Result<&'a Self::Output> {
        match self {
            ReferenceOr::Item(item) => Ok(item),
            ReferenceOr::Reference { reference } => {
                let name = get_parameter_name(reference)?;
                let components = spec
                    .components
                    .as_ref()
                    .ok_or_else(|| anyhow!("no components in spec"))?;
                let param_ref = components
                    .parameters
                    .get(name)
                    .ok_or_else(|| anyhow!("{reference} not found in OpenAPI spec"))?;
                param_ref
                    .as_item()
                    .ok_or_else(|| {
                        let ref_ = param_ref.as_ref_str().expect("param_ref was not item so must be ref");
                        anyhow!("reference {reference} refers to {ref_}").context("references must refer directly to the definition; chains are not permitted")
                    })
            }
        }
    }
}

impl Resolve for ReferenceOr<Response> {
    type Output = Response;

    fn resolve<'a>(&'a self, spec: &'a OpenAPI) -> Result<&'a Self::Output> {
        match self {
            ReferenceOr::Item(item) => Ok(item),
            ReferenceOr::Reference { reference } => {
                let name = get_response_name(reference)?;
                let components = spec
                    .components
                    .as_ref()
                    .ok_or_else(|| anyhow!("no components in spec"))?;
                let response_ref = components
                    .responses
                    .get(name)
                    .ok_or_else(|| anyhow!("{reference} not found in OpenAPI spec"))?;
                response_ref
                    .as_item()
                    .ok_or_else(|| {
                        let ref_ = response_ref.as_ref_str().expect("response_ref was not item so must be ref");
                        anyhow!("reference {reference} refers to {ref_}").context("references must refer directly to the definition; chains are not permitted")
                    })
            }
        }
    }
}

impl Resolve for ReferenceOr<RequestBody> {
    type Output = RequestBody;

    fn resolve<'a>(&'a self, spec: &'a OpenAPI) -> Result<&'a Self::Output> {
        match self {
            ReferenceOr::Item(item) => Ok(item),
            ReferenceOr::Reference { reference } => {
                let name = get_request_body_name(reference)?;
                let components = spec
                    .components
                    .as_ref()
                    .ok_or_else(|| anyhow!("no components in spec"))?;
                let request_body_ref = components
                    .request_bodies
                    .get(name)
                    .ok_or_else(|| anyhow!("{reference} not found in OpenAPI spec"))?;
                request_body_ref
                    .as_item()
                    .ok_or_else(|| {
                        let ref_ = request_body_ref.as_ref_str().expect("request_body_ref was not item so must be ref");
                        anyhow!("reference {reference} refers to {ref_}").context("references must refer directly to the definition; chains are not permitted")
                    })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_request_body_name() {
        assert!(matches!(
            get_request_body_name("#/components/requestBodies/Foo"),
            Ok("Foo")
        ));
        assert!(get_request_body_name("#/components/schemas/Foo").is_err());
    }
}
