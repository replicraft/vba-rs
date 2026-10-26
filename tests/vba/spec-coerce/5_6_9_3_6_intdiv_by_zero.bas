' INV: vbal.5.6.9.3.6, op.intdiv
' SOURCE: spec ms-vbal 5.6.9.3.6 \ Operator, dividing by 0 raises Division by zero
' EXPECT: ok=3
' ERROR: 11
Public Sub Main()
    Dim a As Long, b As Long
    Dim r As Variant
    a = 7: b = 2
    r = a \ b
    Debug.Print "ok=" & CStr(r)
    b = 0
    r = a \ b
    Debug.Print "not reached"
End Sub
