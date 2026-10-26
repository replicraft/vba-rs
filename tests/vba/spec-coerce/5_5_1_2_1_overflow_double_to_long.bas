' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, out-of-range Double to Long is Overflow
' EXPECT: ok=2147483647
' ERROR: 6
Public Sub Main()
    Dim l As Long
    Dim d As Double
    d = 2147483647
    l = d
    Debug.Print "ok=" & CStr(l)
    d = 3000000000#
    l = d
    Debug.Print "not reached"
End Sub
