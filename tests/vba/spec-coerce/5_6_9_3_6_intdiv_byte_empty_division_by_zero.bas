' INV: vbal.5.6.9.3.6, op.intdiv
' SOURCE: spec ms-vbal 5.6.9.3.6 \ Operator, an Empty divisor is treated as 0 so the division raises Division by zero
' ERROR: 11
Public Sub Main()
    Dim e As Variant
    Dim a As Variant
    Dim r As Variant
    a = CByte(5)
    r = a \ e
    Debug.Print "not reached"
End Sub
