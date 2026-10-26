' INV: vbal.5.6.9.3, op.mul
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, Single overflow with no Variant operand raises error 6
' ERROR: 6
Public Sub Main()
    Dim s As Single, t As Single
    Dim r As Variant
    s = 3E+38: t = 100
    r = s * t
    Debug.Print "not reached"
End Sub
