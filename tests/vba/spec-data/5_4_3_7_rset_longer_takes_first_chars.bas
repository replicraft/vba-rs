' INV: vbal.5.4.3.7, stmt.rset
' SOURCE: spec ms-vbal 5.4.3.7 longer value contributes its first qLength characters
' EXPECT: [abc]
' EXPECT: [abc]
Public Sub Main()
    Dim s As String
    s = "123"
    RSet s = "abcdef"
    Debug.Print "[" & s & "]"
    RSet s = "abc"
    Debug.Print "[" & s & "]"
End Sub
