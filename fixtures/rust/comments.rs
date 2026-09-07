// A single line comment.

// A run of line comments
// spanning three lines
// in total.

/* A block comment
   spanning two lines. */

/// A doc comment line
/// continued on a second line.
pub fn documented() -> i32 {
    let value = 1; // trailing comment after code
    // a comment nested inside a function body
    value
}

// a file-scope comment after the function
pub fn simple() {}
