' INV: stmt.lset, stmt.rset
' SOURCE: spec ms-vbal 5.4.3.6 LSet statement and 5.4.3.7 RSet statement
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
