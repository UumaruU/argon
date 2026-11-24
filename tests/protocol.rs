use argon::core::snapshot::{Snapshot, UpdatedSnapshot};
use rbx_dom_weak::{
	types::{Attributes, BinaryString, Tags, Variant},
	ustr, HashMapExt, UstrMap,
};
use rmpv::Value;
use serde::Serialize;
use std::io::Cursor;

fn map_get<'a>(value: &'a Value, key: &str) -> &'a Value {
	value
		.as_map()
		.and_then(|entries| {
			entries
				.iter()
				.find(|(entry_key, _)| entry_key.as_str() == Some(key))
				.map(|(_, entry_value)| entry_value)
		})
		.unwrap_or_else(|| panic!("missing MessagePack key {key:?} in {value:?}"))
}

#[test]
fn snapshot_messagepack_uses_plugin_pod_for_binary_values() {
	let mut attributes = Attributes::new();
	attributes.insert("Blob".into(), BinaryString::from(b"Hello!".to_vec()).into());
	attributes.insert("Enabled".into(), true.into());

	let mut properties = UstrMap::new();
	properties.insert(ustr("Binary"), BinaryString::from(b"Hello!".to_vec()).into());
	properties.insert(ustr("Attributes"), attributes.into());
	properties.insert(ustr("Tags"), Tags::from(vec!["alpha".into(), "beta".into()]).into());

	let snapshot = Snapshot::new().with_properties(properties);
	let bytes = rmp_serde::to_vec_named(&snapshot).expect("snapshot should serialize");
	let payload = rmpv::decode::read_value(&mut Cursor::new(bytes)).expect("payload should be valid MessagePack");
	let properties = map_get(&payload, "properties");

	let binary = map_get(map_get(properties, "Binary"), "BinaryString");
	assert_eq!(binary.as_str(), Some("SGVsbG8h"));

	let attributes = map_get(map_get(properties, "Attributes"), "Attributes");
	let attribute_binary = map_get(map_get(attributes, "Blob"), "BinaryString");
	assert_eq!(attribute_binary.as_str(), Some("SGVsbG8h"));

	let tags = map_get(map_get(properties, "Tags"), "Tags");
	assert_eq!(
		tags.as_array().expect("Tags should be an array"),
		&[Value::from("alpha"), Value::from("beta")]
	);
}

#[test]
fn plugin_pod_messagepack_deserializes_binary_attributes_and_tags() {
	#[derive(Serialize)]
	struct PluginUpdate<'a> {
		id: rbx_dom_weak::types::Ref,
		meta: Option<()>,
		name: Option<&'a str>,
		class: Option<&'a str>,
		properties: Option<serde_json::Value>,
	}

	let payload = PluginUpdate {
		id: rbx_dom_weak::types::Ref::none(),
		meta: None,
		name: None,
		class: None,
		properties: Some(serde_json::json!({
			"Binary": { "BinaryString": "SGVsbG8h" },
			"Attributes": {
				"Attributes": {
					"Blob": { "BinaryString": "SGVsbG8h" },
					"Enabled": { "Bool": true }
				}
			},
			"Tags": { "Tags": ["alpha", "beta"] }
		})),
	};

	let bytes = rmp_serde::to_vec_named(&payload).expect("plugin payload should serialize");
	let update: UpdatedSnapshot = rmp_serde::from_slice(&bytes).expect("plugin payload should deserialize");
	let properties = update.properties.expect("properties should be present");

	assert_eq!(
		properties.get(&ustr("Binary")),
		Some(&Variant::BinaryString(BinaryString::from(b"Hello!".to_vec())))
	);
	let attributes = match properties.get(&ustr("Attributes")) {
		Some(Variant::Attributes(attributes)) => attributes,
		other => panic!("expected Attributes, got {other:?}"),
	};
	assert_eq!(
		attributes.get("Blob"),
		Some(&Variant::BinaryString(BinaryString::from(b"Hello!".to_vec())))
	);
	assert_eq!(attributes.get("Enabled"), Some(&Variant::Bool(true)));

	let tags = match properties.get(&ustr("Tags")) {
		Some(Variant::Tags(tags)) => tags,
		other => panic!("expected Tags, got {other:?}"),
	};
	assert_eq!(tags.iter().collect::<Vec<_>>(), ["alpha", "beta"]);
}
