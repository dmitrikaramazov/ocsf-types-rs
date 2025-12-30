#[test]
fn test_parse_raw_json() {
    let raw_log = include_str!("data/sample_file_activity.json");
    let event: ocsf_types::FileActivity = serde_json::from_str(raw_log).expect("Failed to parse FileActivity log");


    let metadata = event.metadata.as_ref().expect("Metadata is missing");
    let reporter = metadata.reporter.as_ref().expect("Reporter is missing");
    let reporter_name = reporter.name.as_deref().expect("Reporter name is missing");
    let activity_id = event.activity_id.expect("Activity ID is missing");

    assert_eq!(reporter_name, "jul gtk cleaners");
    assert_eq!(activity_id, 12);
    assert_eq!(activity_id, ocsf_types::FileActivityActivityId::Mount as i64);
}

#[test]
fn test_unknown_fields_don_not_break_parsing() {
    let json = r#"{
        "activity_id": 1,
        "class_uid": 1001,
        "new_field": "should be ignored"
    }"#;
    let res: Result<ocsf_types::NetworkActivity, _> = serde_json::from_str(json);
    assert!(res.is_ok());
}

#[test]
fn unmapped_data_is_stored_in_unmapped_field() {
    let json = r#"{
        "activity_id": 1,
        "class_uid": 1001,
        "new_field": "should be ignored",
        "unmapped": {"key1":"value1", "key2":[1,2,3]}
    }"#;
    let res: Result<ocsf_types::NetworkActivity, _> = serde_json::from_str(json);
    // formatting this weird bc ide formatting is acting up
    let event      = res.unwrap();
    let unmapped            = event.unmapped.as_ref()
                                              .expect("Unmapped is missing");
    let unmapped_val1         = unmapped.get("key1")
                                              .expect("key1 is missing")
                                              .as_str()
                                      .expect("key1 is not a string");
    let unmapped_val2  = unmapped.get("key2")
                                              .expect("key2 is missing")
                                              .as_array()
                                              .expect("key2 is not an array");

    assert_eq!(unmapped_val1, "value1");
    assert_eq!(unmapped_val2, &vec![1,2,3]);
}

#[test]
fn deeply_nested_unmapped_data_is_stored_correctly() {
    let json = r#"{
        "activity_id": 1,
        "class_uid": 1001,
        "new_field": "should be ignored",
        "unmapped": {"key1":{"key2":{"key3":"value3", "key4":[1,2,{ "key5":"value5" }]}}}
    }"#;
    let res: Result<ocsf_types::NetworkActivity, _> = serde_json::from_str(json);
    let event = res.unwrap();
    let unmapped = event.unmapped.as_ref().expect("Unmapped is missing");
    // get key5 value
    let val5 = unmapped["key1"]["key2"]["key4"][2]["key5"].as_str().expect("key5 is not a string");
    assert_eq!(val5, "value5");
}


#[test]
fn missing_option_fields_are_not_present_in_the_event() {
    let raw_log = include_str!("data/sample_device_inventory_info_missing_optional.json");
    // activity_name and category_name are optional
    let event: ocsf_types::InventoryInfo = serde_json::from_str(raw_log).expect("Failed to parse InventoryInfo log");
    assert!(event.activity_name.is_none());
    assert!(event.category_name.is_none());
    // just to be safe make sure required fields are present
    assert_eq!(event.category_uid, Some(5));
    assert_eq!(event.activity_id, Some(1));
}

#[test]
fn observables_are_stored_correctly() {
    let raw_log = include_str!("data/sample_device_inventory_info_missing_optional.json");
    let event: ocsf_types::InventoryInfo = serde_json::from_str(raw_log).expect("Failed to parse InventoryInfo log");
    let observables = event.observables.as_ref().expect("Observables are missing");
    let observable = observables.get(0).expect("Observable is missing");
    assert_eq!(observable.name, Some("weeks cam reflects".to_string()));
    assert_eq!(observable.value, Some("alberta dx deliver".to_string()));
    assert_eq!(observable.r#type, Some("Registry Key".to_string()));
    assert_eq!(observable.type_id, Some(28));
    assert_eq!(observable.event_uid, Some("bc63ea5c-e51b-11f0-9e1b-d6ff413579c1".to_string()));
    assert_eq!(observable.reputation.as_ref().expect("Reputation is missing").base_score, Some(86.933));
    assert_eq!(observable.reputation.as_ref().expect("Reputation is missing").provider, Some("mel assume trigger".to_string()));
    assert_eq!(observable.reputation.as_ref().expect("Reputation is missing").score, Some("Malicious".to_string()));
    assert_eq!(observable.reputation.as_ref().expect("Reputation is missing").score_id, Some(10));
}

#[test]
fn round_trip_de_serialization_is_idempotent() {
    let raw_log = include_str!("data/sample_device_inventory_info_missing_optional.json");
    let event: ocsf_types::InventoryInfo = serde_json::from_str(raw_log).expect("Failed to parse InventoryInfo log");
    let serialized = serde_json::to_string(&event).expect("Failed to serialize InventoryInfo log");
    let event2: ocsf_types::InventoryInfo = serde_json::from_str(&serialized).expect("Failed to parse serialized InventoryInfo log");
    assert_eq!(event, event2);
    assert!(event.activity_name.is_none());
    assert!(event2.activity_name.is_none());
    assert_eq!(event.category_uid, Some(5));
    assert_eq!(event2.category_uid, Some(5));
}

#[test]
fn verify_default_values() {
    let mut event = ocsf_types::AccountChange::default();
    assert_eq!(event.activity_id, None);
    assert_eq!(event.class_uid, None);
    assert_eq!(event.message, None);
    assert_eq!(event.metadata, None);
    assert_eq!(event.observables, None);
    assert_eq!(event.unmapped, None);
    assert_eq!(event.activity_name, None);
    assert_eq!(event.category_name, None);
    assert_eq!(event.category_uid, None);
    event.activity_id = Some(1);
    event.class_uid = Some(1001);
    event.message = Some("User password changed".to_string());
    event.metadata = Some(Box::new(ocsf_types::Metadata::default()));
    event.observables = Some(vec![ocsf_types::Observable::default()]);
    event.unmapped = Some(serde_json::json!({}));
    event.activity_name = Some("User password changed".to_string());
    event.category_name = Some("User password changed".to_string());
    event.category_uid = Some(1);
    assert_eq!(event.activity_id, Some(1));
    assert_eq!(event.class_uid, Some(1001));
    assert_eq!(event.message, Some("User password changed".to_string()));
    assert_eq!(event.metadata, Some(Box::new(ocsf_types::Metadata::default())));
    assert_eq!(event.observables, Some(vec![ocsf_types::Observable::default()]));
    assert_eq!(event.unmapped, Some(serde_json::json!({})));
    assert_eq!(event.activity_name, Some("User password changed".to_string()));
    assert_eq!(event.category_name, Some("User password changed".to_string()));
    assert_eq!(event.category_uid, Some(1));
}

#[test]
fn test_various_ways_to_create_an_event(){
    use ocsf_types::AccountChange;
    // Note - you should ensure that all required fields exist
    let mut event = AccountChange::default();
    event.activity_id = Some(1);
    let event2 = {
        let mut e = AccountChange::default();
        e.activity_id = Some(1);
        e
    };
    let event3: AccountChange = serde_json::from_value(
        serde_json::json!({
            "activity_id": 1,
        })
    ).expect("Failed to parse AccountChange log");

    assert_eq!(event2, event3);
    assert_eq!(event, event2);

    let serialized = serde_json::to_string(&event)
                             .expect("Failed to serialize AccountChange log 1");
    let serialized2 = serde_json::to_string(&event2)
                             .expect("Failed to serialize AccountChange log 2");
    let serialized3 = serde_json::to_string(&event3)
                             .expect("Failed to serialize AccountChange log 3");

    assert_eq!(serialized, serialized2);
    assert_eq!(serialized2, serialized3);
}

#[test]
fn extract_nested_data_functional() {
    let raw_log = include_str!("data/sample_file_activity.json");
    let event: ocsf_types::FileActivity = serde_json::from_str(raw_log).expect("Failed to parse FileActivity log");
    let file_result_ext
            = event
              .file_result
              .as_ref()
              .and_then(|file_result| file_result.ext.as_deref());
    assert_eq!(file_result_ext, Some("ranked review reverse"));
}

#[test]
fn test_http_activity() {
    let raw_log = include_str!("data/sample_http_activity.json");
    let event: ocsf_types::HttpActivity = serde_json::from_str(raw_log).expect("Failed to parse HttpActivity log");
    let http_response = event.http_response.as_ref().expect("HTTP Response is missing");
    assert_eq!(http_response.code, Some(44));

    assert_eq!(event.activity_id, Some(1));
    assert_eq!(event.activity_id, Some(ocsf_types::HttpActivityActivityId::Connect as i64));
    assert_eq!(event.activity_id_enum(), Some(ocsf_types::HttpActivityActivityId::Connect));

    let http_headers = http_response.http_headers.as_ref().expect("HTTP Headers are missing");
    assert_eq!(http_headers.len(), 2);
    assert_eq!(http_headers[0].name, Some("hung due ongoing".to_string()));
    assert_eq!(http_headers[0].value, Some("accounts budgets minister".to_string()));
    assert_eq!(http_headers[1].name, Some("examinations liabilities scholars".to_string()));
    assert_eq!(http_headers[1].value, Some("cpu his marilyn".to_string()));

    let timespan = event.traffic.as_ref().and_then(|traffic| traffic.timespan.as_ref());
    assert_eq!(timespan.and_then(|t| t.type_id_enum()), Some(ocsf_types::TimespanTypeId::Weeks));
    assert_eq!(timespan.and_then(|t| t.type_id), Some(ocsf_types::TimespanTypeId::Weeks as i64));
    assert_eq!(timespan.and_then(|t| t.type_id), Some(6));
    assert_eq!(timespan.and_then(|t| t.duration), Some(3296457038));

}