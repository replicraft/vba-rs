' INV: vbal.5.6.9.5, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational non-Error with Error operand raises 13
' XFAIL: wrong pending
' EXPECT: before
' ERROR: 13
Public Sub Main()
    Dim a As Variant, r As Variant
    a = CVErr(3)
    Debug.Print "before"
    r = ("x" < a)
    Debug.Print "after"
End Sub
