' INV: vbal.5.6.9.3.5, op.div
' SOURCE: spec ms-vbal 5.6.9.3.5 / Operator, Empty / Empty is 0 / 0 with integral-style operands and raises Overflow
' XFAIL: wrong pending
' ERROR: 6
Public Sub Main()
    Dim e As Variant
    Dim f As Variant
    Dim r As Variant
    r = e / f
    Debug.Print "not reached"
End Sub
