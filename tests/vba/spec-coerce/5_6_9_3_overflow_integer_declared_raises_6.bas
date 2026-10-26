' INV: vbal.5.6.9.3, op.mul
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, overflow with no Variant operand raises error 6
' EXPECT: ok=30000
' ERROR: 6
Public Sub Main()
    Dim a As Integer, b As Integer
    Dim r As Variant
    a = 30000: b = 1
    r = a * b
    Debug.Print "ok=" & CStr(r)
    b = 2
    r = a * b
    Debug.Print "not reached"
End Sub
