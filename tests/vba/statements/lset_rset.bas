' INV: stmt.lset, stmt.rset
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/lset-statement
' EXPECT: [ab    ]
' EXPECT: [    ab]
' EXPECT: [abcdef]
Public Sub Main()
    Dim s As String * 6
    LSet s = "ab"
    Debug.Print "[" & s & "]"
    RSet s = "ab"
    Debug.Print "[" & s & "]"
    LSet s = "abcdefgh"
    Debug.Print "[" & s & "]"
End Sub
