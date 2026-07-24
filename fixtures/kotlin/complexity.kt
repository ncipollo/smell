fun simple() {
    println("hi")
}

fun branchy(x: Int): Int {
    if (x > 0) {
        return 1
    } else if (x < -10) {
        return -2
    }
    for (i in 0 until x) {
        while (i > 2) {
            break
        }
    }
    var y = x
    do {
        y--
    } while (y > 5)
    when (x) {
        1 -> return 1
        2 -> return 2
        else -> {}
    }
    val label: String? = null
    val fallback = label ?: "none"
    if (y > 1 && y < 100 || x == -5) {
        return y
    }
    return try {
        canThrow()
        fallback.length
    } catch (error: Exception) {
        -1
    }
}

fun canThrow() {}

class Shape(var width: Int) {
    var count: Int = 0

    val area: Int
        get() {
            return if (width > 0) width * width else 0
        }

    var label: String = ""
        get() = if (field.isEmpty()) "none" else field
        set(value) {
            if (value.isNotEmpty()) {
                field = value
            }
        }

    init {
        if (width < 0) {
            width = 0
        }
    }

    constructor(width: Int, count: Int) : this(width) {
        if (count > 0) {
            this.count = count
        }
    }

    fun describe(): String {
        return if (area > 10) "big" else "small"
    }

    companion object {
        fun unit(): Shape {
            return Shape(1)
        }
    }
}

object Registry {
    fun register(shape: Shape): Boolean {
        return shape.area > 0
    }
}

interface Labeled {
    fun label(size: Int): String {
        return if (size > 10) "big" else "small"
    }
}
