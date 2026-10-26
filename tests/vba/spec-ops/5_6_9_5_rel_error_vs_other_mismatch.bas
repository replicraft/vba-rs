' INV: vbal.5.6.9.5, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational Error with non-Error operand raises 13
' XFAIL: wrong pending
' EXPECT: before
' ERROR: 13
Public Sub Main()
    Dim a As Variant, r As Variant
    a = CVErr(3)
    Debug.Print "before"
    r = (a = 3)
    Debug.Print "after"
End Sub
