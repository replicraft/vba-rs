' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: spec ms-vbal 5.4.3.6 expression Let-coerced to String
' EXPECT: [42   ]
Public Sub Main()
    Dim s As String
    s = "12345"
    LSet s = 42
    Debug.Print "[" & s & "]"
End Sub
