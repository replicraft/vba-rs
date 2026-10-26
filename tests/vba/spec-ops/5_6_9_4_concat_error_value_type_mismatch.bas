' INV: vbal.5.6.9.4, op.concat
' SOURCE: spec ms-vbal 5.6.9.4 & operator, Error value type raises 13
' XFAIL: wrong pending
' EXPECT: before
' ERROR: 13
Public Sub Main()
    Dim s As String
    Dim v As Variant
    v = CVErr(5)
    Debug.Print "before"
    s = "a" & v
    Debug.Print "after"
End Sub
