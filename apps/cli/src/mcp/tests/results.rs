use crate::mcp::inputs::ResponseFormat;
use crate::mcp::results::{Skills, result, schema};
use serde_json::json;
use skillbinder_proto::command_line::{Output, PresetRow, SkillOutput};

#[test]
fn a_list_stops_at_200_and_says_how_many_more() {
    let presets = (0..203)
        .map(|index| PresetRow {
            name: format!("Preset {index}"),
            active: index == 0,
            skills: 1,
        })
        .collect();
    let listed = result(Ok(Output::Presets(presets)), ResponseFormat::Concise);
    let structured = listed.structured_content.unwrap_or_default();
    assert_eq!(structured["presets"].as_array().map(Vec::len), Some(200));
    assert_eq!(structured["more"], 3);
    assert!(
        structured.get("folders").is_none(),
        "only the kind asked for"
    );
    let text = listed.content[0].as_text().map(|text| text.text.clone());
    assert!(text.is_some_and(|text| text.ends_with("…and 3 more.\n")));
}

#[test]
fn concise_leaves_out_descriptions_sources_and_recorded_ids() {
    let page = SkillOutput {
        skill: super::skill("pdf"),
        source: None,
        recorded: "4b825dc6".into(),
        presets: vec!["Default".into()],
        copies: Vec::new(),
    };
    let output = || Output::Skill(Box::new(page.clone()));
    let concise = result(Ok(output()), ResponseFormat::Concise).structured_content;
    let detailed = result(Ok(output()), ResponseFormat::Detailed).structured_content;
    let (concise, detailed) = (concise.unwrap_or_default(), detailed.unwrap_or_default());
    for detail in ["description", "source", "recorded"] {
        assert!(concise.get(detail).is_none(), "{detail}");
        assert!(detailed.get(detail).is_some(), "{detail}");
    }
    assert_eq!(concise["presets"], json!(["Default"]));
}

#[test]
fn schemas_leave_the_detail_fields_optional_and_drop_integer_formats() {
    let skills = schema::<Skills>();
    let row = &skills["$defs"]["SkillRow"]["required"];
    assert!(
        row.as_array()
            .is_some_and(|required| required.contains(&json!("slug")))
    );
    assert!(
        !row.as_array()
            .is_some_and(|required| required.contains(&json!("description")))
    );
    assert!(
        !skills.contains_key("format")
            && !skills["properties"]["more"]
                .as_object()
                .is_some_and(|more| more.contains_key("format"))
    );
    let page = schema::<SkillOutput>();
    for detail in ["description", "source", "recorded"] {
        assert!(
            !page["required"]
                .as_array()
                .is_some_and(|required| required.contains(&json!(detail)))
        );
    }
}
