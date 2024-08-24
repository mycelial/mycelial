use config::prelude::*;

#[derive(Configuration)]
#[section(input)]
struct Config0{
    
}

#[derive(Configuration)]
#[section(input=)]
struct Config1{
    
}

#[derive(Configuration)]
#[section(output)]
struct Config2{
    
}

#[derive(Configuration)]
#[section(output=)]
struct Config3{
    
}

#[derive(Configuration)]
#[section(input=foo)]
struct Config4{
    
}

#[derive(Configuration)]
#[section(input=baz)]
struct Config5{
    
}

#[derive(Configuration)]
#[section(inut=typo)]
struct Config6{
    
}

#[derive(Configuration)]
#[section(input-bin)]
struct Config7{
    
}

fn main() {}