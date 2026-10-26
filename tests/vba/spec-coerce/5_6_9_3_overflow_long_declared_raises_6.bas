' INV: vbal.5.6.9.3, op.add
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, Long + Long overflow with no Variant operand raises error 6
' ERROR: 6
Public Sub Main()
    Dim a As Long, b As Long
    Dim r As Variant
    a = 2147483647: b = 1
    r = a + b
    Debug.Print "not reached"
End Sub
