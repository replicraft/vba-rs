' INV: stmt.lset, stmt.rset
' SOURCE: spec ms-vbal 5.4.3.6 LSet statement, 5.4.3.7 RSet statement
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
