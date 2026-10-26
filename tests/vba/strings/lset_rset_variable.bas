' INV: stmt.lset, stmt.rset
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/lset-statement
' EXPECT: [xy   ]
' EXPECT: [   xy]
Public Sub Main()
    Dim s As String
    s = "abcde"
    LSet s = "xy"
    Debug.Print "[" & s & "]"
    s = "abcde"
    RSet s = "xy"
    Debug.Print "[" & s & "]"
End Sub
