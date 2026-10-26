' INV: vbal.5.6.9.3, op.mul
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, Currency overflow raises error 6 even with a Variant operand
' ERROR: 6
Public Sub Main()
    Dim v As Variant
    Dim r As Variant
    v = CCur(900000000000000#)
    r = v * v
    Debug.Print "not reached"
End Sub
