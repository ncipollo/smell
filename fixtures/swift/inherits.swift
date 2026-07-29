protocol Describe {
    func describe() -> String
}

class Base {
    func area() -> Int {
        return 1
    }
}

class Circle: Base, Describe {
    func describe() -> String {
        return "circle"
    }
}

struct Plain {
    func value() -> Int {
        return 1
    }
}

class Container<T> {
    func first() -> T? {
        return nil
    }
}

class Wide: Container<Int> {
    func widen() -> Int {
        return 2
    }
}

extension Circle: Swift.Equatable {}
