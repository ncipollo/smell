fn simple() {
    println!("hi");
}

fn branchy(x: i32) -> i32 {
    if x > 0 {
        return 1;
    } else if x < -10 {
        return -2;
    }
    let Some(y) = maybe(x) else {
        return 0;
    };
    for i in 0..x {
        while i > 2 {
            break;
        }
    }
    loop {
        break;
    }
    match x {
        1 => return 1,
        2 => return 2,
        _ => {}
    }
    if x > 1 && x < 100 || x == -5 {
        return y;
    }
    y
}

fn fallible(x: i32) -> Result<i32, String> {
    let value = parse(x)?;
    Ok(value)
}

fn maybe(x: i32) -> Option<i32> {
    Some(x)
}

fn parse(x: i32) -> Result<i32, String> {
    Ok(x)
}

struct Shape {
    width: i32,
    height: i32,
}

impl Shape {
    fn area(&self) -> i32 {
        if self.width > 0 { self.width * self.height } else { 0 }
    }
}

impl std::fmt::Display for Shape {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.area() > 10 {
            write!(formatter, "big")
        } else {
            write!(formatter, "small")
        }
    }
}

enum Kind {
    Circle,
    Square,
}

impl Kind {
    fn label(&self) -> &'static str {
        match self {
            Kind::Circle => "circle",
            Kind::Square => "square",
        }
    }
}

trait Describe {
    fn describe(&self) -> String {
        String::from("shape")
    }
}
