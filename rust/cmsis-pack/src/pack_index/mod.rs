use crate::utils::prelude::*;
use anyhow::Error;
use minidom::Element;

#[derive(Debug, Clone)]
pub struct PdscRef {
    pub url: String,
    pub vendor: String,
    pub name: String,
    pub version: String,
    pub date: Option<String>,
    pub deprecated: Option<String>,
    pub replacement: Option<String>,
    pub size: Option<String>,
}

#[derive(Debug)]
pub struct Pidx {
    pub url: String,
    pub vendor: String,
    pub date: Option<String>,
}

#[derive(Debug)]
pub struct Vidx {
    pub vendor: String,
    pub url: String,
    pub timestamp: Option<String>,
    pub pdsc_index: Vec<PdscRef>,
    pub vendor_index: Vec<Pidx>,
}

impl FromElem for PdscRef {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "pdsc")?;
        Ok(Self {
            url: attr_map(e, "url", "pdsc")?,
            vendor: attr_map(e, "vendor", "pdsc")?,
            name: attr_map(e, "name", "pdsc")?,
            version: attr_map(e, "version", "pdsc")?,
            date: attr_map(e, "date", "pdsc").ok(),
            deprecated: attr_map(e, "deprecated", "pdsc").ok(),
            replacement: attr_map(e, "replacement", "pdsc").ok(),
            size: attr_map(e, "size", "pdsc").ok(),
        })
    }
}

impl FromElem for Pidx {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "pidx")?;
        Ok(Self {
            url: attr_map(e, "url", "pidx")?,
            vendor: attr_map(e, "vendor", "pidx")?,
            date: attr_map(e, "date", "pidx").ok(),
        })
    }
}

impl FromElem for Vidx {
    fn from_elem(root: &Element) -> Result<Self, Error> {
        assert_root_name(root, "index")?;
        let vendor = child_text(root, "vendor", "index")?;
        let url = child_text(root, "url", "index")?;
        Ok(Vidx {
            vendor,
            url,
            timestamp: get_child_no_ns(root, "timestamp").map(Element::text),
            vendor_index: get_child_no_ns(root, "vindex")
                .map(|e| Pidx::vec_from_children(e.children()))
                .unwrap_or_default(),
            pdsc_index: get_child_no_ns(root, "pindex")
                .map(|e| PdscRef::vec_from_children(e.children()))
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pdscref_missing_attr() {
        let erroring_strings = vec![
            "<pdsc>",
            "<pdsc url=\"Vendor\" name=\"Name\" version=\"1.2.3-alpha\">",
            "<pdsc vendor=\"Vendor\" url=\"Url\" version=\"1.2.3-alpha\">",
            "<pdsc vendor=\"Vendor\" name=\"Name\" url=\"Url\">",
            "<pdsc vendor=\"Vendor\" name=\"Name\" version=\"1.2.3-alpha\">",
        ];
        for bad_string in erroring_strings {
            assert!(PdscRef::from_string(bad_string).is_err());
        }
    }

    #[test]
    fn pdscref_wrong_elem() {
        let bad_string =
            "<notPdsc vendor=\"Vendor\" url=\"Url\" name=\"name\" version=\"1.2.3-alpha\">";
        assert!(PdscRef::from_string(bad_string).is_err())
    }

    #[test]
    fn pdscref_optionals() {
        let good_string =
            "<pdsc vendor=\"Vendor\" url=\"Url\" name=\"Name\" version=\"1.2.3-alpha\">";
        let response = PdscRef::from_string(good_string).unwrap();
        assert_eq!(response.vendor, String::from("Vendor"));
        assert_eq!(response.url, "Url");
        assert_eq!(response.name, String::from("Name"));
        assert_eq!(response.version, String::from("1.2.3-alpha"));
        let good_string =
            "<pdsc vendor=\"Vendor\" url=\"Url\" name=\"Name\" version=\"1.2.3-alpha\"
                date=\"A-Date\" deprecated=\"true\" replacement=\"Other\" size=\"8MB\">";
        let response = PdscRef::from_string(good_string).unwrap();
        assert_eq!(response.date, Some(String::from("A-Date")));
        assert_eq!(response.deprecated, Some(String::from("true")));
        assert_eq!(response.replacement, Some(String::from("Other")));
        assert_eq!(response.size, Some(String::from("8MB")));
    }

    #[test]
    fn pidx_misssing_attr() {
        let erroring_strings = vec![
            "<pidx/>",
            "<pidx vendor=\"Vendor\"/>",
            "<pidx url=\"Url\"/>",
        ];
        for bad_string in erroring_strings {
            assert!(Pidx::from_string(bad_string).is_err());
        }
    }

    #[test]
    fn pidx_wrong_elem() {
        let bad_string = "<notpidx url=\"Url\" vendor=\"Vendor\"/>";
        assert!(Pidx::from_string(bad_string).is_err())
    }

    #[test]
    fn pidx_optionals() {
        let good_string = "<pidx vendor=\"Vendor\" url=\"Url\"/>";
        let response = Pidx::from_string(good_string).unwrap();
        assert_eq!(response.vendor, String::from("Vendor"));
        assert_eq!(response.url, "Url");

        let good_string =
            "<pidx vendor=\"Vendor\" url=\"Url\" date=\"Fri Sep  1 11:21:06 CDT 2017\"/>";
        let response = Pidx::from_string(good_string).unwrap();
        assert_eq!(response.vendor, String::from("Vendor"));
        assert_eq!(response.url, "Url");
        assert_eq!(
            response.date,
            Some(String::from("Fri Sep  1 11:21:06 CDT 2017"))
        )
    }

    #[test]
    fn vidx_misssing_attr() {
        let erroring_strings = vec![
            "<index xmlns:xs=\"http://www.w3.org/2001/XMLSchema-instance\">
             </index>",
            "<index xmlns:xs=\"http://www.w3.org/2001/XMLSchema-instance\">
               <vendor>Vendor</vendor>
             </index>",
            "<index xmlns:xs=\"http://www.w3.org/2001/XMLSchema-instance\">
               <url>Url</url>
             </index>",
        ];
        for bad_string in erroring_strings {
            assert!(Vidx::from_string(bad_string).is_err());
        }
    }

    #[test]
    fn vidx_wrong_elem() {
        let bad_string = "<notindex xmlns:xs=\"http://www.w3.org/2001/XMLSchema-instance\">
               <vendor>Vendor</vendor>
               <url>Url</url>
             </notindex>";
        assert!(Vidx::from_string(bad_string).is_err())
    }

    #[test]
    fn vidx_optionals() {
        let good_string = "<index xmlns:xs=\"http://www.w3.org/2001/XMLSchema-instance\">
               <vendor>Vendor</vendor>
               <url>Url</url>
             </index>";
        let response = Vidx::from_string(good_string).unwrap();
        assert_eq!(response.vendor, String::from("Vendor"));
        assert_eq!(response.url, "Url");

        let good_string = "<index xmlns:xs=\"http://www.w3.org/2001/XMLSchema-instance\">
               <vendor>Vendor</vendor>
               <url>Url</url>
               <timestamp>Fri Sep  1 13:26:41 CDT 2017</timestamp>
             </index>";
        let response = Vidx::from_string(good_string).unwrap();
        assert_eq!(response.vendor, String::from("Vendor"));
        assert_eq!(response.url, "Url");
    }
}
