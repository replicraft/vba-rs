' INV: vbal.5.6.9.3.5, op.div
' SOURCE: spec ms-vbal 5.6.9.3.5 / Operator, 0 divided by Empty with an Integer left operand raises Overflow (error 6)
' ERROR: 6
Public Sub Main()
    Dim a As Integer
    Dim e As Variant
    Dim r As Variant
    a = 0
    r = a / e
    Debug.Print "not reached"
End Sub
