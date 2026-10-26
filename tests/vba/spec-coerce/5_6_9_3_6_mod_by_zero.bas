' INV: vbal.5.6.9.3.6, op.mod
' SOURCE: spec ms-vbal 5.6.9.3.6 Mod Operator, dividing by 0 raises Division by zero
' EXPECT: ok=1
' ERROR: 11
Public Sub Main()
    Dim a As Long, b As Long
    Dim r As Variant
    a = 7: b = 2
    r = a Mod b
    Debug.Print "ok=" & CStr(r)
    b = 0
    r = a Mod b
    Debug.Print "not reached"
End Sub
