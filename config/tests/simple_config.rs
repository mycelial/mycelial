use config::{Config as _, Field, FieldType, Metadata, SectionConfig};

#[allow(unused)]
#[derive(SectionConfig)]
struct SimpleConfig {
    login: String,
    #[input(password, text_area)]
    password: String,
    port: u16,
    bool: bool,
}

#[test]
fn test() {
    let cfg = SimpleConfig {
        login: "login".into(),
        password: "password".into(),
        port: 30303,
        bool: true,
    };
    println!("{:#?}", cfg.fields());
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
