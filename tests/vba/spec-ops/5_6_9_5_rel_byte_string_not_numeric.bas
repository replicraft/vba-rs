' INV: vbal.5.6.9.5, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational Byte vs String coerces the String to Byte (type mismatch)
' EXPECT: before
' ERROR: 13
Public Sub Main()
    Dim b As Byte, r As Boolean
    b = 5
    Debug.Print "before"
    r = (b = "abc")
    Debug.Print "after"
End Sub
