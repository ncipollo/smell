// A single line comment.

// A run of line comments
// spanning three lines
// in total.

/* A block comment
   spanning two lines. */

/// <summary>
/// An XML doc comment.
/// </summary>
public class Comments
{
    public int Value()
    {
        int value = 1; // trailing comment after code
        // a comment nested inside a method body
        return value;
    }
}

// a file-scope comment after the class
