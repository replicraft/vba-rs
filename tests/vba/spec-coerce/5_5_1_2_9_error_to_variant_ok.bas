' INV: vbal.5.5.1.2.9, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.9 Let-coercion to and from Error, an Error value may be held in a Variant
' EXPECT: iserror=True
' EXPECT: copy iserror=True
' EXPECT: typename=Error
Public Sub Main()
    Dim v As Variant
    Dim w As Variant
    v = CVErr(2042)
    w = v
    Debug.Print "iserror=" & CStr(IsError(v))
    Debug.Print "copy iserror=" & CStr(IsError(w))
    Debug.Print "typename=" & TypeName(w)
End Sub
