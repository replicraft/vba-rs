' INV: vbal.5.4.3.7, stmt.rset
' SOURCE: spec ms-vbal 5.4.3.7 declared type may be Variant holding a String
' EXPECT: [  ab]
Public Sub Main()
    Dim v As Variant
    v = "1234"
    RSet v = "ab"
    Debug.Print "[" & v & "]"
End Sub
