interface Describe {
    fun describe(): String {
        return "thing"
    }
}

open class Base(x: Int) {
    fun area(): Int {
        return 1
    }
}

class Circle : Base(1), Describe {
    override fun describe(): String {
        return "circle"
    }
}

class Plain {
    fun value(): Int {
        return 1
    }
}

class Ranked : Comparable<Ranked> {
    override fun compareTo(other: Ranked): Int {
        return 0
    }
}

object Registry : Describe {
    fun count(): Int {
        return 0
    }
}
