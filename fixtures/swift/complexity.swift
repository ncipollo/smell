func canThrow() throws {}

func simple() {
    print("hi")
}

func branchy(x: Int) -> Int {
    if x > 0 {
        return 1
    } else if x < -10 {
        return -2
    }
    guard x != 0 else { return 0 }
    for i in 0..<x {
        while i > 2 {
            break
        }
    }
    repeat {
        print("loop")
    } while x > 5
    switch x {
    case 1:
        return 1
    case 2:
        return 2
    default:
        break
    }
    let y = x > 3 ? 1 : 0
    let optional: Int? = nil
    let z = optional ?? y
    if z > 1 && z < 100 || z == -5 {
        return y
    }
    do {
        try canThrow()
    } catch {
        return -1
    }
    return y
}

struct Shape {
    var width = 0
    var height = 0

    var area: Int {
        return width > 0 ? width * height : 0
    }

    var label: String {
        get {
            if area > 10 { return "big" }
            return "small"
        }
        set {
            guard !newValue.isEmpty else { return }
            print(newValue)
        }
    }

    var count: Int = 0 {
        willSet {
            if newValue > 10 { print("big") }
        }
        didSet {
            guard count != oldValue else { return }
            print("changed")
        }
    }
}

extension Shape {
    func describe() -> String {
        if area > 10 { return "big" }
        return "small"
    }
}
