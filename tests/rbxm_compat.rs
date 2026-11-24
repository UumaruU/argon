use rbx_dom_weak::{types::Variant, ustr};

fn roundtrip(dom: &rbx_dom_weak::WeakDom) -> rbx_dom_weak::WeakDom {
	let mut output = Vec::new();
	rbx_binary::to_writer(&mut output, dom, dom.root().children()).expect("fixture should serialize back to binary");
	rbx_binary::from_reader(output.as_slice()).expect("serialized fixture should deserialize again")
}

const CURRENT_STUDIO_MODEL: &[u8] = include_bytes!("fixtures/argon-probe.rbxm");
const EXISTING_MODEL: &[u8] = include_bytes!("fixtures/Meadow_01.rbxm");

#[test]
fn current_studio_model_decodes_shared_string_tags() {
	let dom = rbx_binary::from_reader(CURRENT_STUDIO_MODEL)
		.expect("a model saved by current Roblox Studio should deserialize");
	let model = dom
		.root()
		.children()
		.first()
		.and_then(|referent| dom.get_by_ref(*referent))
		.expect("fixture should contain a root model");

	assert_eq!(model.name, "ArgonProbe");
	assert_eq!(model.class, "Model");
	let tags = match model.properties.get(&ustr("Tags")) {
		Some(Variant::Tags(tags)) => tags,
		other => panic!("expected current Studio Tags property, got {other:?}"),
	};
	assert!(tags.is_empty(), "probe fixture should have no assigned tags");

	let part = model
		.children()
		.first()
		.and_then(|referent| dom.get_by_ref(*referent))
		.expect("fixture model should contain a part");
	assert_eq!(part.class, "Part");

	let roundtripped = roundtrip(&dom);
	let roundtripped_model = roundtripped
		.root()
		.children()
		.first()
		.and_then(|referent| roundtripped.get_by_ref(*referent))
		.expect("round-tripped fixture should retain its root model");
	let roundtripped_tags = match roundtripped_model.properties.get(&ustr("Tags")) {
		Some(Variant::Tags(tags)) => tags,
		other => panic!("expected round-tripped Tags property, got {other:?}"),
	};
	assert_eq!(roundtripped_tags, tags);
}

#[test]
fn existing_meadow_model_still_deserializes() {
	let dom = rbx_binary::from_reader(EXISTING_MODEL).expect("existing supported models should keep deserializing");
	let root = dom
		.root()
		.children()
		.first()
		.and_then(|referent| dom.get_by_ref(*referent))
		.expect("fixture should contain a root instance");

	assert!(!root.name.is_empty());
	assert!(!root.children().is_empty());

	let roundtripped = roundtrip(&dom);
	assert_eq!(roundtripped.root().children().len(), dom.root().children().len());
}
