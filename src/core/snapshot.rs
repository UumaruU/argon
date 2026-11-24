use rbx_dom_weak::{
	types::{Ref, Variant},
	ustr, HashMapExt, Ustr, UstrMap,
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Formatter};

use super::{helpers::apply_migrations, meta::Meta};
use crate::{middleware::data::DataSnapshot, Properties};

#[derive(Clone, Serialize, Deserialize)]
pub struct Snapshot {
	pub id: Ref,
	pub meta: Meta,

	// Roblox related
	pub name: String,
	pub class: Ustr,
	#[serde(with = "wire_properties")]
	pub properties: Properties,
	pub children: Vec<Snapshot>,
}

impl Snapshot {
	// Creating new snapshot

	pub fn new() -> Self {
		Self {
			id: Ref::none(),
			meta: Meta::new(),
			name: String::new(),
			class: Ustr::from("Folder"),
			properties: UstrMap::new(),
			children: Vec::new(),
		}
	}

	pub fn with_id(mut self, id: Ref) -> Self {
		self.set_id(id);
		self
	}

	pub fn with_meta(mut self, meta: Meta) -> Self {
		self.set_meta(meta);
		self
	}

	pub fn with_name(mut self, name: &str) -> Self {
		self.set_name(name);
		self
	}

	pub fn with_class(mut self, class: &str) -> Self {
		self.set_class(class);
		self
	}

	pub fn with_properties(mut self, properties: Properties) -> Self {
		self.set_properties(properties);
		self
	}

	pub fn with_children(mut self, children: Vec<Snapshot>) -> Self {
		self.set_children(children);
		self
	}

	pub fn with_data(mut self, data: DataSnapshot) -> Self {
		self.apply_data(data);
		self
	}

	// Overwriting snapshot fields

	pub fn set_id(&mut self, id: Ref) {
		self.id = id;
	}

	pub fn set_meta(&mut self, meta: Meta) {
		self.meta = meta;
	}

	pub fn set_name(&mut self, name: &str) {
		name.clone_into(&mut self.name);
	}

	pub fn set_class(&mut self, class: &str) {
		self.class = class.into();
	}

	pub fn set_properties(&mut self, properties: Properties) {
		self.properties = properties;
		apply_migrations(&self.class, &mut self.properties);
	}

	pub fn set_children(&mut self, children: Vec<Snapshot>) {
		self.children = children;
	}

	pub fn apply_data(&mut self, data: DataSnapshot) {
		if let Some(class) = data.class {
			self.class = class;
		}

		if let Some(keep_unknowns) = data.keep_unknowns {
			self.meta.keep_unknowns = keep_unknowns;
		}

		if let Some(original_name) = data.original_name {
			self.name = original_name.clone();
			self.meta.set_original_name(Some(original_name));
		}

		if let Some(mesh_source) = data.mesh_source {
			self.meta.set_mesh_source(Some(mesh_source));
		}

		self.extend_properties(data.properties);
		self.meta.source.add_data(&data.path);
	}

	// Adding to snapshot fields

	pub fn add_property(&mut self, name: &str, value: Variant) {
		self.properties.insert(name.into(), value);
		apply_migrations(&self.class, &mut self.properties);
	}

	pub fn add_child(&mut self, child: Snapshot) {
		self.children.push(child);
	}

	// Joining snapshot fields

	pub fn extend_properties(&mut self, properties: Properties) {
		self.properties.extend(properties);
		apply_migrations(&self.class, &mut self.properties);
	}

	pub fn extend_children(&mut self, children: Vec<Snapshot>) {
		self.children.extend(children);
	}

	// Miscellaneous

	pub fn as_new(&self, parent: Ref) -> AddedSnapshot {
		AddedSnapshot {
			id: self.id,
			meta: self.meta.clone(),
			parent,
			name: self.name.clone(),
			class: self.class,
			properties: self.properties.clone(),
			children: self.children.clone(),
		}
	}
}

impl Debug for Snapshot {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let mut debug = f.debug_struct("Snapshot");

		debug.field("name", &self.name);
		debug.field("class", &self.class);
		debug.field("id", &self.id);
		debug.field("meta", &self.meta);

		if !self.properties.is_empty() {
			let mut properties = self.properties.clone();

			if let Some(property) = properties.get_mut(&ustr("Source")) {
				if let Variant::String(source) = property {
					let lines = source.lines().count();

					if lines > 1 {
						*property = Variant::String(format!("Truncated... ({lines} lines)"));
					}
				}
			}

			debug.field("properties", &properties);
		}

		if !self.children.is_empty() {
			debug.field("children", &self.children);
		}

		debug.finish()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddedSnapshot {
	pub id: Ref,
	pub meta: Meta,
	pub parent: Ref,
	pub name: String,
	pub class: Ustr,
	#[serde(with = "wire_properties")]
	pub properties: Properties,
	pub children: Vec<Snapshot>,
}

impl From<AddedSnapshot> for Snapshot {
	fn from(snapshot: AddedSnapshot) -> Self {
		Self {
			id: snapshot.id,
			meta: snapshot.meta,
			name: snapshot.name,
			class: snapshot.class,
			properties: snapshot.properties,
			children: snapshot.children,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatedSnapshot {
	pub id: Ref,
	pub meta: Option<Meta>,
	pub name: Option<String>,
	pub class: Option<Ustr>,
	#[serde(default, with = "wire_properties::optional")]
	pub properties: Option<Properties>,
}

mod wire_properties {
	use serde::{de::Error as _, ser::Error as _, Deserialize, Deserializer, Serialize, Serializer};

	use crate::Properties;

	pub fn serialize<S>(properties: &Properties, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serde_json::to_value(properties)
			.map_err(S::Error::custom)?
			.serialize(serializer)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Properties, D::Error>
	where
		D: Deserializer<'de>,
	{
		let value = serde_json::Value::deserialize(deserializer)?;
		serde_json::from_value(value).map_err(D::Error::custom)
	}

	pub mod optional {
		use super::*;

		pub fn serialize<S>(properties: &Option<Properties>, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			properties
				.as_ref()
				.map(serde_json::to_value)
				.transpose()
				.map_err(S::Error::custom)?
				.serialize(serializer)
		}

		pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Properties>, D::Error>
		where
			D: Deserializer<'de>,
		{
			Option::<serde_json::Value>::deserialize(deserializer)?
				.map(serde_json::from_value)
				.transpose()
				.map_err(D::Error::custom)
		}
	}
}

impl UpdatedSnapshot {
	pub fn new(id: Ref) -> Self {
		Self {
			id,
			name: None,
			class: None,
			properties: None,
			meta: None,
		}
	}

	pub fn is_empty(&self) -> bool {
		self.meta.is_none() && self.name.is_none() && self.class.is_none() && self.properties.is_none()
	}
}
