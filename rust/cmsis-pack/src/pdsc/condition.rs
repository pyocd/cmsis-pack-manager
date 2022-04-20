use anyhow::Error;
use minidom::Element;

use crate::utils::prelude::*;

pub struct ConditionComponent {
    pub device_family: Option<String>,
    pub device_sub_family: Option<String>,
    pub device_variant: Option<String>,
    pub device_vendor: Option<String>,
    pub device_name: Option<String>,
}

impl FromElem for ConditionComponent {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        Ok(ConditionComponent {
            device_family: attr_map(e, "Dfamily", "condition").ok(),
            device_sub_family: attr_map(e, "Dsubfamily", "condition").ok(),
            device_variant: attr_map(e, "Dvariant", "condition").ok(),
            device_vendor: attr_map(e, "Dvendor", "condition").ok(),
            device_name: attr_map(e, "Dname", "condition").ok(),
        })
    }
}

pub struct Condition {
    pub id: String,
    pub accept: Vec<ConditionComponent>,
    pub deny: Vec<ConditionComponent>,
    pub require: Vec<ConditionComponent>,
}

impl FromElem for Condition {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "condition")?;
        let mut accept = Vec::new();
        let mut deny = Vec::new();
        let mut require = Vec::new();
        for elem in e.children() {
            match elem.name() {
                "accept" => {
                    accept.push(ConditionComponent::from_elem(e)?);
                }
                "deny" => {
                    deny.push(ConditionComponent::from_elem(e)?);
                }
                "require" => {
                    require.push(ConditionComponent::from_elem(e)?);
                }
                "description" => {}
                _ => {
                    log::warn!("Found unkonwn element {} in components", elem.name());
                }
            }
        }
        Ok(Condition {
            id: attr_map(e, "id", "condition")?,
            accept,
            deny,
            require,
        })
    }
}

#[derive(Default)]
pub struct Conditions(pub Vec<Condition>);

impl FromElem for Conditions {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "conditions")?;
        Ok(Conditions(
            e.children()
                .flat_map(|c| Condition::from_elem(c).ok_warn())
                .collect(),
        ))
    }
}
