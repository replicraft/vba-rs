' INV: vbal.5.4.3.7, stmt.rset
' SOURCE: spec ms-vbal 5.4.3.7 shorter value right-aligned with leading spaces
' EXPECT: [   ab]
Public Sub Main()
    Dim s As String
    s = "12345"
    RSet s = "ab"
    Debug.Print "[" & s & "]"
End Sub
