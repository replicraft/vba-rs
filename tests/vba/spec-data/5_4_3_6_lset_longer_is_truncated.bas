' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: spec ms-vbal 5.4.3.6 longer value truncated to initial characters
' EXPECT: [abc]
' EXPECT: [abc]
Public Sub Main()
    Dim s As String
    s = "123"
    LSet s = "abcdef"
    Debug.Print "[" & s & "]"
    LSet s = "abc"
    Debug.Print "[" & s & "]"
End Sub
