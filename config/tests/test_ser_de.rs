use config::prelude::*;

#[test]
fn test_serialization_deserialization() {
    #[derive(Debug, Clone, config::Config, Default, PartialEq)]
    struct Config {
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
    }

    let cfg: Box<dyn config::Config> = Box::new(Config {
        i8: -8,
        i16: -16,
        i32: -32,
        i64: -64,
        u8: 8,
        u16: 16,
        u32: 32,
        u64: 64,
        bool: true,
        string: "some string".into(),
    });

    let serialized = serde_json::to_string(&cfg).unwrap();
    let deserialized = serde_json::from_str::<Box<dyn config::Config>>(&serialized);
    assert!(deserialized.is_ok(), "{:?}", deserialized);
    let deserialized = deserialized.unwrap();
    let mut fields = deserialized.fields();
    fields.sort_by_key(|field| field.name);
    assert_eq!(
        fields,
        vec![
            Field {
                name: "bool",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: true,
                },
                value: FieldValue::Bool(true)
            },
            Field {
                name: "i16",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: true,
                },
                value: FieldValue::I64(-16)
            },
            Field {
                name: "i32",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: true,
                },
                value: FieldValue::I64(-32)
            },
            Field {
                name: "i64",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: true,
                },
                value: FieldValue::I64(-64)
            },
            Field {
                name: "i8",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: true,
                },
                value: FieldValue::I64(-8)
            },
            Field {
                name: "string",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: true,
                },
                value: FieldValue::String("some string")
            },
            Field {
                name: "u16",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: true,
                },
                value: FieldValue::U64(16)
            },
            Field {
                name: "u32",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: true,
                },
                value: FieldValue::U64(32)
            },
            Field {
                name: "u64",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: true,
                },
                value: FieldValue::U64(64)
            },
            Field {
                name: "u8",
                ty: FieldType::String,
                metadata: Metadata {
                    is_password: false,
                    is_text_area: false,
                    is_read_only: true,
                },
                value: FieldValue::U64(8)
            }
        ]
    );
    let mut cfg2: Box<dyn config::Config> = Box::new(Config::default());
    deserialize_into_config(&*deserialized, &mut *cfg2).unwrap();
    let cfg = unsafe { &*(&*cfg as *const _ as *const () as *const Config) };
    let cfg2 = unsafe { &*(&*cfg2 as *const _ as *const () as *const Config) };
    assert_eq!(cfg, cfg2);
}
