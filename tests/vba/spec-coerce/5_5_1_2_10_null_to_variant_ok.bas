' INV: vbal.5.5.1.2.10, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.10 Let-coercion from Null, Null may be copied to another Variant
' EXPECT: isnull=True
' EXPECT: typename=Null
Public Sub Main()
    Dim v As Variant
    Dim w As Variant
    v = Null
    w = v
    Debug.Print "isnull=" & CStr(IsNull(w))
    Debug.Print "typename=" & TypeName(w)
End Sub
