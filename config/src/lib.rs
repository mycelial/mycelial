use config_derive::Config;

pub trait Config {
    fn fields(&self) -> Vec<Field>;
}

#[derive(Debug)]
pub enum FieldType {
    Int,
    String,
    Bool,
}

#[derive(Debug)]
pub struct Field {
    pub name: &'static str,
    pub ty: FieldType
}


#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug, Config)]
    struct TestConfig {
        #[output]
        login: String,
        #[input(type=password)]
        password: String,    
    }
    
    #[test]
    fn test() {
        let t = TestConfig{login: "login".into(), password: "password".into()};
    }
}