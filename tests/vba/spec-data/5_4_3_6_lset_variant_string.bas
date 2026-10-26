' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: spec ms-vbal 5.4.3.6 declared type may be Variant holding a String
' EXPECT: [ab   ]
Public Sub Main()
    Dim v As Variant
    v = "12345"
    LSet v = "ab"
    Debug.Print "[" & v & "]"
End Sub
