use config::prelude::*;

#[test]
fn test_simple_config() {
    #[derive(Debug, Config)]
    #[section(input=dataframe, output=dataframe)]
    struct SimpleConfig {
        login: String,
        #[field_type(password, text_area)]
        password: String,
        port: u16,
        bool: bool,
    }

    let cfg = SimpleConfig {
        login: "login".into(),
        password: "password".into(),
        port: 30303,
        bool: true,
    };
    assert_eq!(cfg.name(), "SimpleConfig");
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
                value: "login".into(),
            },
            Field {
                name: "password",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: true,
                    is_text_area: true,
                },
                value: "password".into(),
            },
            Field {
                name: "port",
                ty: FieldType::U16,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                },
                value: 30303_u16.into(),
            },
            Field {
                name: "bool",
                ty: FieldType::Bool,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                },
                value: true.into()
            },
        ],
    )
}

#[test]
fn test_config_all_types() {
    #[derive(Debug, Config)]
    struct _Config {
        i8: i8,
        i16: i16,
        i32: i32,
        i64: i64,
        u8: u8,
        u16: u16,
        u32: u32,
        u64: u64,
        bool: bool,
        string: String,
        //v: Vec<String>,
    }
}

#[test]
fn test_section_input_output() {
    #[derive(Debug, Config)]
    struct NoInput {}
    assert_eq!(NoInput {}.input(), SectionIO::None);
    assert_eq!(NoInput {}.output(), SectionIO::None);

    #[derive(Debug, Config)]
    #[section(input=bin)]
    struct InputBin {}
    assert_eq!(InputBin {}.input(), SectionIO::Bin);
    assert_eq!(InputBin {}.output(), SectionIO::None);

    #[derive(Debug, Config)]
    #[section(input=dataframe)]
    struct InputDf {}
    assert_eq!(InputDf {}.input(), SectionIO::DataFrame);
    assert_eq!(InputDf {}.output(), SectionIO::None);

    #[derive(Debug, Config)]
    #[section(output=bin)]
    struct OutputBin {}
    assert_eq!(OutputBin {}.input(), SectionIO::None);
    assert_eq!(OutputBin {}.output(), SectionIO::Bin);

    #[derive(Debug, Config)]
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