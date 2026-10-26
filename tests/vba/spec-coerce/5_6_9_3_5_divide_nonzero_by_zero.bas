' INV: vbal.5.6.9.3.5, op.div
' SOURCE: spec ms-vbal 5.6.9.3.5 / Operator, a nonzero value divided by 0 raises Division by zero
' EXPECT: ok=2
' ERROR: 11
Public Sub Main()
    Dim a As Integer, b As Integer
    Dim r As Variant
    a = 4: b = 2
    r = a / b
    Debug.Print "ok=" & CStr(r)
    b = 0
    r = a / b
    Debug.Print "not reached"
End Sub
