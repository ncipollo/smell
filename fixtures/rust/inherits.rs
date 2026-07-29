trait Describe {
    fn describe(&self) -> String {
        "thing".to_string()
    }
}

struct Circle {
    radius: f64,
}

impl Circle {
    fn radius(&self) -> f64 {
        self.radius
    }
}

impl Describe for Circle {
    fn describe(&self) -> String {
        "circle".to_string()
    }
}

impl std::fmt::Display for Circle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "circle")
    }
}

struct Plain;

impl Plain {
    fn value(&self) -> usize {
        1
    }
}

struct Wrapper(i32);

impl From<i32> for Wrapper {
    fn from(value: i32) -> Wrapper {
        Wrapper(value)
    }
}

struct Marked;

impl Marked {
    fn mark(&self) -> bool {
        true
    }
}

impl Describe for Marked {}
