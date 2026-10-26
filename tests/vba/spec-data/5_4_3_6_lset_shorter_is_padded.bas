' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: spec ms-vbal 5.4.3.6 shorter value padded with spaces on the right
' EXPECT: [ab   ]
Public Sub Main()
    Dim s As String
    s = "12345"
    LSet s = "ab"
    Debug.Print "[" & s & "]"
End Sub
