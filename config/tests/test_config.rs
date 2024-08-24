use config::prelude::*;
use quickcheck::{quickcheck, TestResult};

#[test]
fn test_simple_config() {
    #[derive(Debug, Clone, Configuration)]
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
                    is_read_only: false,
                },
                value: "login".into(),
            },
            Field {
                name: "password",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: true,
                    is_text_area: true,
                    is_read_only: false,
                },
                value: "password".into(),
            },
            Field {
                name: "port",
                ty: FieldType::U16,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: false,
                },
                value: 30303_u16.into(),
            },
            Field {
                name: "bool",
                ty: FieldType::Bool,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: false,
                },
                value: true.into()
            },
        ],
    )
}

#[test]
fn test_config_all_types() {
    #[derive(Debug, Clone, Configuration)]
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
    #[derive(Debug, Clone, Configuration)]
    struct NoInput {}
    assert_eq!(NoInput {}.input(), SectionIO::None);
    assert_eq!(NoInput {}.output(), SectionIO::None);

    #[derive(Debug, Clone, Configuration)]
    #[section(input=bin)]
    struct InputBin {}
    assert_eq!(InputBin {}.input(), SectionIO::Bin);
    assert_eq!(InputBin {}.output(), SectionIO::None);

    #[derive(Debug, Clone, Configuration)]
    #[section(input=dataframe)]
    struct InputDf {}
    assert_eq!(InputDf {}.input(), SectionIO::DataFrame);
    assert_eq!(InputDf {}.output(), SectionIO::None);

    #[derive(Debug, Clone, Configuration)]
    #[section(output=bin)]
    struct OutputBin {}
    assert_eq!(OutputBin {}.input(), SectionIO::None);
    assert_eq!(OutputBin {}.output(), SectionIO::Bin);

    #[derive(Debug, Clone, Configuration)]
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

// test combination of types over config set field
#[test]
fn test_set_field() {
    #[derive(Debug, Clone, Default, Configuration, PartialEq)]
    struct Conf{
        bool: bool,
        i8: i8,
        i16: i16,
        i32: i32,
        i64: i64,
        u8: u8,
        u16: u16,
        u32: u32,
        u64: u64,
        string: String,
    }
    fn check<T>(t: T) -> TestResult
    where
        for<'a> T: ToString
            + Into<FieldValue<'a>>
            + Clone
            + TryInto<i8>
            + TryInto<i16>
            + TryInto<i32>
            + TryInto<i64>
            + TryInto<u8>
            + TryInto<u16>
            + TryInto<u32>
            + TryInto<u64>,
    {
        let mut config = Box::new(Conf::default());
        let mut config_from_strings = Box::new(Conf::default());
        let str = t.to_string();
        let string_field_value: FieldValue = (&str).into();

        match <T as TryInto<i8>>::try_into(t.clone()) {
            Ok(val) => {
                let new_field_value: FieldValue = t.clone().into();
                assert!(config.set_field_value("i8", new_field_value).is_ok());
                let expected_field_value: FieldValue = val.into();
                let current_field_value_res = config.get_field_value("i8");
                assert!(current_field_value_res.is_ok());
                let current_field_value = current_field_value_res.unwrap();
                assert_eq!(expected_field_value, current_field_value);
                assert_eq!(current_field_value.field_type(), FieldType::I8);
                assert!(config_from_strings
                    .set_field_value("i8", string_field_value)
                    .is_ok());
            }
            Err(_) => {
                assert!(config_from_strings
                    .set_field_value("i8", string_field_value)
                    .is_err());
            }
        };
        match <T as TryInto<i16>>::try_into(t.clone()) {
            Ok(val) => {
                let new_field_value: FieldValue = t.clone().into();
                assert!(config.set_field_value("i16", new_field_value).is_ok());
                let expected_field_value: FieldValue = val.into();
                let current_field_value_res = config.get_field_value("i16");
                assert!(current_field_value_res.is_ok());
                let current_field_value = current_field_value_res.unwrap();
                assert_eq!(expected_field_value, current_field_value);
                assert_eq!(current_field_value.field_type(), FieldType::I16);
                assert!(config_from_strings
                    .set_field_value("i16", string_field_value)
                    .is_ok());
            }
            Err(_) => {
                assert!(config_from_strings
                    .set_field_value("i16", string_field_value)
                    .is_err());
            }
        };
        match <T as TryInto<i32>>::try_into(t.clone()) {
            Ok(val) => {
                let new_field_value: FieldValue = t.clone().into();
                assert!(config.set_field_value("i32", new_field_value).is_ok());
                let expected_field_value: FieldValue = val.into();
                let current_field_value_res = config.get_field_value("i32");
                assert!(current_field_value_res.is_ok());
                let current_field_value = current_field_value_res.unwrap();
                assert_eq!(expected_field_value, current_field_value);
                assert_eq!(current_field_value.field_type(), FieldType::I32);
                assert!(config_from_strings
                    .set_field_value("i32", string_field_value)
                    .is_ok());
            }
            Err(_) => {
                assert!(config_from_strings
                    .set_field_value("i32", string_field_value)
                    .is_err());
            }
        };
        match <T as TryInto<i64>>::try_into(t.clone()) {
            Ok(val) => {
                let new_field_value: FieldValue = t.clone().into();
                assert!(config.set_field_value("i64", new_field_value).is_ok());
                let expected_field_value: FieldValue = val.into();
                let current_field_value_res = config.get_field_value("i64");
                assert!(current_field_value_res.is_ok());
                let current_field_value = current_field_value_res.unwrap();
                assert_eq!(expected_field_value, current_field_value);
                assert_eq!(current_field_value.field_type(), FieldType::I64);
                assert!(config_from_strings
                    .set_field_value("i64", string_field_value)
                    .is_ok());
            }
            Err(_) => {
                assert!(config_from_strings
                    .set_field_value("i64", string_field_value)
                    .is_err());
            }
        };
        match <T as TryInto<u8>>::try_into(t.clone()) {
            Ok(val) => {
                let new_field_value: FieldValue = t.clone().into();
                assert!(config.set_field_value("u8", new_field_value).is_ok());
                let expected_field_value: FieldValue = val.into();
                let current_field_value_res = config.get_field_value("u8");
                assert!(current_field_value_res.is_ok());
                let current_field_value = current_field_value_res.unwrap();
                assert_eq!(expected_field_value, current_field_value);
                assert!(config_from_strings
                    .set_field_value("u8", string_field_value)
                    .is_ok());
            }
            Err(_) => {
                assert!(config_from_strings
                    .set_field_value("u8", string_field_value)
                    .is_err());
            }
        };
        match <T as TryInto<u16>>::try_into(t.clone()) {
            Ok(val) => {
                let new_field_value: FieldValue = t.clone().into();
                assert!(config.set_field_value("u16", new_field_value).is_ok());
                let expected_field_value: FieldValue = val.into();
                let current_field_value_res = config.get_field_value("u16");
                assert!(current_field_value_res.is_ok());
                let current_field_value = current_field_value_res.unwrap();
                assert_eq!(expected_field_value, current_field_value);
                assert_eq!(current_field_value.field_type(), FieldType::U16);
                assert!(config_from_strings
                    .set_field_value("u16", string_field_value)
                    .is_ok());
            }
            Err(_) => {
                assert!(config_from_strings
                    .set_field_value("u16", string_field_value)
                    .is_err());
            }
        };
        match <T as TryInto<u32>>::try_into(t.clone()) {
            Ok(val) => {
                let new_field_value: FieldValue = t.clone().into();
                assert!(config.set_field_value("u32", new_field_value).is_ok());
                let expected_field_value: FieldValue = val.into();
                let current_field_value_res = config.get_field_value("u32");
                assert!(current_field_value_res.is_ok());
                let current_field_value = current_field_value_res.unwrap();
                assert_eq!(expected_field_value, current_field_value);
                assert_eq!(current_field_value.field_type(), FieldType::U32);
                assert!(config_from_strings
                    .set_field_value("u32", string_field_value)
                    .is_ok());
            }
            Err(_) => {
                assert!(config_from_strings
                    .set_field_value("u32", string_field_value)
                    .is_err());
            }
        };
        match <T as TryInto<u64>>::try_into(t.clone()) {
            Ok(val) => {
                let new_field_value: FieldValue = t.clone().into();
                assert!(config.set_field_value("u64", new_field_value).is_ok());
                let expected_field_value: FieldValue = val.into();
                let current_field_value_res = config.get_field_value("u64");
                assert!(current_field_value_res.is_ok());
                let current_field_value = current_field_value_res.unwrap();
                assert_eq!(expected_field_value, current_field_value);
                assert_eq!(current_field_value.field_type(), FieldType::U64);
                assert!(config_from_strings
                    .set_field_value("u64", string_field_value)
                    .is_ok());
            }
            Err(_) => {
                assert!(config_from_strings
                    .set_field_value("u64", string_field_value)
                    .is_err());
            }
        };

        let bool = str == "0";
        let str = bool.to_string();
        let string_field_value: FieldValue = (&str).into();
        let new_field_value: FieldValue = bool.into();
        assert!(config.set_field_value("bool", new_field_value).is_ok());
        let expected_field_value: FieldValue = bool.into();
        let current_field_value_res = config.get_field_value("bool");
        assert!(current_field_value_res.is_ok());
        let current_field_value = current_field_value_res.unwrap();
        assert_eq!(expected_field_value, current_field_value);
        assert_eq!(current_field_value.field_type(), FieldType::Bool);
        assert!(config_from_strings
            .set_field_value("bool", string_field_value)
            .is_ok());

        // config set from type should be equal config set from string values of that type
        assert_eq!(
            unsafe { &*((&*config) as *const _ as *const () as *const Conf) },
            unsafe { &*((&*config_from_strings) as *const _ as *const () as *const Conf) },
        );
        TestResult::from_bool(true)
    }
    quickcheck(check::<i8> as fn(i8) -> TestResult);
    quickcheck(check::<i16> as fn(i16) -> TestResult);
    quickcheck(check::<i32> as fn(i32) -> TestResult);
    quickcheck(check::<i64> as fn(i64) -> TestResult);
    quickcheck(check::<u8> as fn(u8) -> TestResult);
    quickcheck(check::<u16> as fn(u16) -> TestResult);
    quickcheck(check::<u32> as fn(u32) -> TestResult);
    quickcheck(check::<u64> as fn(u64) -> TestResult);
}
