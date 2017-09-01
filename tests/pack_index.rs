extern crate cmsis;
extern crate smallstring;

use cmsis::pack_index::*;
use cmsis::parse::FromElem;
use smallstring::SmallString;

#[test]
fn pdscref_missing_attr(){
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
fn pdscref_wrong_elem(){
    let bad_string =
        "<notPdsc vendor=\"Vendor\" url=\"Url\" name=\"name\" version=\"1.2.3-alpha\">";
    assert!(PdscRef::from_string(bad_string).is_err())
}

#[test]
fn pdscref_optionals(){
    let good_string =
        "<pdsc vendor=\"Vendor\" url=\"Url\" name=\"Name\" version=\"1.2.3-alpha\">";
    let response = PdscRef::from_string(good_string).unwrap();
    assert_eq!(response.vendor, SmallString::from("Vendor"));
    assert_eq!(response.url, "Url");
    assert_eq!(response.name, SmallString::from("Name"));
    assert_eq!(response.version, SmallString::from("1.2.3-alpha"));
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
fn pidx_misssing_attr(){
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
fn pidx_wrong_elem(){
    let bad_string =
        "<notpidx url=\"Url\" vendor=\"Vendor\"/>";
    assert!(Pidx::from_string(bad_string).is_err())
}

#[test]
fn pidx_optionals(){
    let good_string =
        "<vidx vendor=\"Vendor\" url=\"Url\"/>";
    let response = Pidx::from_string(good_string).unwrap();
    assert_eq!(response.vendor, SmallString::from("Vendor"));
    assert_eq!(response.url, "Url");

    let good_string =
        "<vidx vendor=\"Vendor\" url=\"Url\" date=\"Fri Sep  1 11:21:06 CDT 2017\"/>";
    let response = Pidx::from_string(good_string).unwrap();
    assert_eq!(response.vendor, SmallString::from("Vendor"));
    assert_eq!(response.url, "Url");
    assert_eq!(response.date, Some(String::from("Fri Sep  1 11:21:06 CDT 2017")))
}
