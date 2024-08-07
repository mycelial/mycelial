use config::prelude::*;

#[derive(Config)]
#[section(input)]
struct Config0 {
    
}

#[derive(Config)]
#[section(input=)]
struct Config1 {
    
}

#[derive(Config)]
#[section(output)]
struct Config2 {
    
}

#[derive(Config)]
#[section(output=)]
struct Config3 {
    
}

#[derive(Config)]
#[section(input=foo)]
struct Config4 {
    
}

#[derive(Config)]
#[section(input=baz)]
struct Config5 {
    
}

#[derive(Config)]
#[section(inut=typo)]
struct Config6 {
    
}

#[derive(Config)]
#[section(input-bin)]
struct Config7 {
    
}

fn main() {}