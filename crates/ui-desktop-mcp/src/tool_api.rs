use rdf_analytics::domain::type_index::{ReferenceCharacteristics, TypeInstanceIndex};
use rdf_glance::domain::visual_query::FilterType;
use rdf_glance::{
    IriIndex,
    domain::{RdfData, graph_model::short_iri},
};
use serde::{Deserialize, Serialize};

use rmcp::schemars::JsonSchema;

pub const DATASET_PROFILE_RESOURCE_URI: &str = "dataset://current/profile";

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct DatasetProfile {
    pub nodes: usize,
    pub unique_predicates: usize,
    pub unique_types: usize,
    pub properties: usize,
    pub references: usize,
    pub blank_nodes: usize,
    pub unresolved_references: usize,
    pub min_instance_type_count: usize,
    pub max_instance_type_count: usize,
    pub predicates: Vec<PredicateProfile>,
    pub types: Vec<TypeProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PredicateProfile {
    pub iri: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct TypeProfile {
    pub iri: String,
    pub label: String,
    pub instance_count: usize,
    pub properties: Vec<PropertyProfile>,
    pub references: Vec<ReferenceProfile>,
    pub reverse_references: Vec<ReferenceProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PropertyProfile {
    pub iri: String,
    pub label: String,
    pub count: u32,
    pub max_length: u32,
    pub min_cardinality: u32,
    pub max_cardinality: u32,
    pub value_types: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReferenceProfile {
    pub iri: String,
    pub label: String,
    pub count: u32,
    pub min_cardinality: u32,
    pub max_cardinality: u32,
    pub reference_types: Vec<String>,
}

pub fn dataset_profile_from_index(rdf_data: &RdfData, type_index: &TypeInstanceIndex) -> DatasetProfile {
    let node_data = &rdf_data.node_data;
    let mut predicates: Vec<PredicateProfile> = type_index
        .predicates
        .iter()
        .filter_map(|predicate_index| {
            let iri = node_data.get_predicate(*predicate_index)?;
            Some(PredicateProfile {
                iri: iri.to_string(),
                label: short_iri(iri).to_string(),
            })
        })
        .collect();
    predicates.sort_by(|a, b| a.iri.cmp(&b.iri));

    let mut types: Vec<TypeProfile> = type_index
        .types
        .iter()
        .filter_map(|(type_index, type_data)| {
            let iri = node_data.get_type(*type_index)?;
            let mut properties: Vec<PropertyProfile> = type_data
                .properties
                .iter()
                .filter_map(|(predicate_index, characteristics)| {
                    let iri = node_data.get_predicate(*predicate_index)?;
                    Some(PropertyProfile {
                        iri: iri.to_string(),
                        label: short_iri(iri).to_string(),
                        count: characteristics.count,
                        max_length: characteristics.max_len,
                        min_cardinality: characteristics.min_cardinality,
                        max_cardinality: characteristics.max_cardinality,
                        value_types: value_type_names(characteristics.value_types),
                    })
                })
                .collect();
            properties.sort_by(|a, b| a.iri.cmp(&b.iri));

            Some(TypeProfile {
                iri: iri.to_string(),
                label: short_iri(iri).to_string(),
                instance_count: type_data.instances.len(),
                properties,
                references: reference_profiles(node_data, &type_data.references),
                reverse_references: reference_profiles(node_data, &type_data.rev_references),
            })
        })
        .collect();
    types.sort_by(|a, b| a.iri.cmp(&b.iri));

    DatasetProfile {
        nodes: type_index.nodes,
        unique_predicates: type_index.unique_predicates,
        unique_types: type_index.unique_types,
        properties: type_index.properties,
        references: type_index.references,
        blank_nodes: type_index.blank_nodes,
        unresolved_references: type_index.unresolved_references,
        min_instance_type_count: type_index.min_instance_type_count,
        max_instance_type_count: type_index.max_instance_type_count,
        predicates,
        types,
    }
}

fn reference_profiles(
    node_data: &rdf_glance::domain::NodeData,
    references: &std::collections::HashMap<IriIndex, ReferenceCharacteristics>,
) -> Vec<ReferenceProfile> {
    let mut profiles: Vec<ReferenceProfile> = references
        .iter()
        .filter_map(|(predicate_index, characteristics)| {
            let iri = node_data.get_predicate(*predicate_index)?;
            let reference_types = characteristics
                .types
                .iter()
                .filter_map(|type_index| node_data.get_type(*type_index).map(|iri| iri.to_string()))
                .collect();
            Some(ReferenceProfile {
                iri: iri.to_string(),
                label: short_iri(iri).to_string(),
                count: characteristics.count,
                min_cardinality: characteristics.min_cardinality,
                max_cardinality: characteristics.max_cardinality,
                reference_types,
            })
        })
        .collect();
    profiles.sort_by(|a, b| a.iri.cmp(&b.iri));
    profiles
}

pub(crate) fn value_type_names(value_types: rdf_glance::domain::ValueTypes) -> Vec<String> {
    let mut names = Vec::new();
    for (flag, name) in [
        (rdf_glance::domain::ValueTypes::STRING, "string"),
        (rdf_glance::domain::ValueTypes::SHORT_STRING, "short_string"),
        (rdf_glance::domain::ValueTypes::LANG_STRING, "lang_string"),
        (rdf_glance::domain::ValueTypes::INTEGER, "integer"),
        (rdf_glance::domain::ValueTypes::DOUBLE, "double"),
        (rdf_glance::domain::ValueTypes::XML, "xml"),
        (rdf_glance::domain::ValueTypes::JSON, "json"),
        (rdf_glance::domain::ValueTypes::DATE_TIME, "date_time"),
        (rdf_glance::domain::ValueTypes::DATE, "date"),
        (rdf_glance::domain::ValueTypes::TIME, "time"),
        (rdf_glance::domain::ValueTypes::DURATION, "duration"),
        (rdf_glance::domain::ValueTypes::BOOLEAN, "boolean"),
        (rdf_glance::domain::ValueTypes::UNKNOWN, "unknown"),
    ] {
        if value_types.contains(flag) {
            names.push(name.to_string());
        }
    }
    names
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InstanceQueryResult {
    pub count: usize,
    pub columns: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InstanceQueryRangeResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NodeInfo {
    pub iri: String,
    pub types: Vec<String>,
    pub properties: Vec<NodePropertyInfo>,
    pub references: Vec<NodeReferenceInfo>,
    pub reverse_references: Vec<NodeReferenceInfo>,
    pub is_blank_node: bool,
    pub has_subject: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NodePropertyInfo {
    pub predicate: String,
    pub value: String,
    pub datatype: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NodeReferenceInfo {
    pub predicate: String,
    pub node_iri: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Filter {
    #[serde(alias = "atrribute")]
    pub attribute: String,
    pub value: String,
    pub operator: FilterType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_operator_is_a_json_enum_string() {
        let filter = Filter {
            attribute: "name".to_string(),
            value: "Alice".to_string(),
            operator: FilterType::Equals,
        };

        let json = serde_json::to_value(&filter).expect("filter should serialize");
        assert_eq!(json["operator"], "Equals");

        let deserialized: Filter = serde_json::from_value(json).expect("filter should deserialize");
        assert_eq!(deserialized.operator, FilterType::Equals);
    }

    #[test]
    fn filter_operator_schema_lists_string_variants() {
        let schema =
            serde_json::to_value(schemars::schema_for!(FilterType)).expect("filter operator schema should serialize");

        assert_eq!(schema["type"], "string");
        assert_eq!(
            schema["enum"],
            serde_json::json!([
                "Contains",
                "Equals",
                "EqualsNoCase",
                "StartsWith",
                "EndsWith",
                "GraterThan",
                "LessThan",
                "Exists",
                "NotExists"
            ])
        );
    }
}
