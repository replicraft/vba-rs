' INV: vbal.5.6.9.8, op.and
' SOURCE: spec ms-vbal 5.6.9.8 logical operator with Error value type raises 13
' XFAIL: wrong pending
' EXPECT: before
' ERROR: 13
Public Sub Main()
    Dim e As Variant, r As Variant
    e = CVErr(7)
    Debug.Print "before"
    r = (e And 1)
    Debug.Print "after"
End Sub
