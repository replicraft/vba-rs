' INV: vbal.5.5.1.2.7, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.7 Let-coercion to and from non-Byte arrays, a number value to a resizable array is Type mismatch at run time
' ERROR: 13
Public Sub Main()
    Dim a() As Long
    Dim v As Variant
    v = 5
    a = v
    Debug.Print "not reached"
End Sub
