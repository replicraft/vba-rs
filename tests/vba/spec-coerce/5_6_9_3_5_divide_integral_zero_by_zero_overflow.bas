' INV: vbal.5.6.9.3.5, op.div
' SOURCE: spec ms-vbal 5.6.9.3.5 / Operator, 0 divided by 0 with integral operands also raises Overflow
' XFAIL: wrong pending
' ERROR: 6
Public Sub Main()
    Dim a As Long, b As Long
    Dim r As Variant
    a = 0: b = 0
    r = a / b
    Debug.Print "not reached"
End Sub
