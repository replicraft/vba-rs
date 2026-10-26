' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, out-of-range Single to Long is Overflow
' ERROR: 6
Public Sub Main()
    Dim l As Long
    Dim s As Single
    s = 3E+10
    l = s
    Debug.Print "not reached"
End Sub
