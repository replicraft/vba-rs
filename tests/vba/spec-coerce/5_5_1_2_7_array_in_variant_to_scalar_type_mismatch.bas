' INV: vbal.5.5.1.2.7, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.7 Let-coercion to and from non-Byte arrays, an array value to a non-array declared type is Type mismatch at run time
' EXPECT: stored
' ERROR: 13
Public Sub Main()
    Dim a() As Long
    Dim v As Variant
    Dim n As Long
    ReDim a(1)
    v = a
    Debug.Print "stored"
    n = v
    Debug.Print "not reached"
End Sub
