// A single line comment.

// A run of line comments
// spanning three lines
// in total.

/* A block comment
   spanning two lines. */

/**
 * A KDoc comment.
 */
class Comments {
    fun value(): Int {
        val value = 1 // trailing comment after code
        // a comment nested inside a function body
        return value
    }
}

// a file-scope comment after the class
