use config::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize)]
#[section(input=dataframe, output=dataframe)]
#[serde(deny_unknown_fields)]
struct SimpleConfig {
    login: String,
    #[field_type(password, text_area)]
    password: String,
    port: u16,
    bool: bool,
}

#[test]
fn test_simple_config() {
    let cfg = SimpleConfig {
        login: "login".into(),
        password: "password".into(),
        port: 30303,
        bool: true,
    };
    assert_eq!(cfg.input(), SectionIO::DataFrame);
    assert_eq!(cfg.output(), SectionIO::DataFrame);
    assert_eq!(
        cfg.fields(),
        vec![
            Field {
                name: "login",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                },
            },
            Field {
                name: "password",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: true,
                    is_text_area: true,
                },
            },
            Field {
                name: "port",
                ty: FieldType::U16,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                },
            },
            Field {
                name: "bool",
                ty: FieldType::Bool,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                },
            },
        ],
    )
}

#[test]
fn test_section_input_output() {
    #[derive(Config)]
    struct NoInput {}
    assert_eq!(NoInput {}.input(), SectionIO::None);
    assert_eq!(NoInput {}.output(), SectionIO::None);

    #[derive(Config)]
    #[section(input=bin)]
    struct InputBin {}
    assert_eq!(InputBin {}.input(), SectionIO::Bin);
    assert_eq!(InputBin {}.output(), SectionIO::None);

    #[derive(Config)]
    #[section(input=dataframe)]
    struct InputDf {}
    assert_eq!(InputDf {}.input(), SectionIO::DataFrame);
    assert_eq!(InputDf {}.output(), SectionIO::None);

    #[derive(Config)]
    #[section(output=bin)]
    struct OutputBin {}
    assert_eq!(OutputBin {}.input(), SectionIO::None);
    assert_eq!(OutputBin {}.output(), SectionIO::Bin);

    #[derive(Config)]
    #[section(output=dataframe)]
    struct OutputDf {}
    assert_eq!(OutputDf {}.input(), SectionIO::None);
    assert_eq!(OutputDf {}.output(), SectionIO::DataFrame);
}

#[test]
fn test_compilations() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compilation_fails_checks/*.rs");
}
